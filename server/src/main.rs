mod api;
mod config;
mod db;
mod error;
mod sync;

use std::sync::Arc;

use clap::Parser;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::str::FromStr;
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};
use tracing::Level;

use crate::{api::AppState, config::Cli};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,ootle_community_templates=debug".into()),
        )
        .init();

    let cli = Cli::parse();

    if cli.create_config {
        match config::Config::write_default(&cli.config) {
            Ok(()) => {
                println!("Config written to {}", cli.config.display());
                return Ok(());
            }
            Err(config::ConfigError::AlreadyExists(path)) => {
                anyhow::bail!("Config file already exists: {}", path.display());
            }
            Err(e) => return Err(e.into()),
        }
    }

    let config = config::Config::load(&cli)?;

    tracing::info!("Connecting to database...");
    let opts = SqliteConnectOptions::from_str(&config.database.url)?.create_if_missing(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(opts)
        .await?;

    // Run migrations
    tracing::info!("Running database migrations...");
    sqlx::migrate!("../migrations").run(&pool).await?;

    // Seed initial admin if none exist
    let admin_count = db::admins::count(&pool).await?;
    if admin_count == 0 {
        tracing::info!("No admins found, creating initial admin user...");
        let hash = api::auth::hash_password(&config.admin.initial_password)
            .map_err(|e| anyhow::anyhow!("Failed to hash initial admin password: {:?}", e))?;
        db::admins::create_admin(&pool, &config.admin.initial_username, &hash).await?;
        tracing::info!("Initial admin '{}' created", config.admin.initial_username);
    }

    // Resolve the effective indexer URL and JWT secret
    let indexer_url = config.indexer_url().to_string();
    let jwt_secret = config.jwt_secret();
    tracing::info!("Using indexer at {indexer_url}");
    if config.server.jwt_secret.is_none() {
        tracing::info!(
            "No jwt_secret configured, using random secret (JWTs invalidated on restart)"
        );
    }

    // Shared coordination primitives between the sync loop and the admin
    // reindex handler. The mutex serialises wipes against in-flight sync
    // passes; the notify lets the reindex handler wake the loop immediately.
    let sync_lock = Arc::new(tokio::sync::Mutex::new(()));
    let reindex_notify = Arc::new(tokio::sync::Notify::new());

    // Start indexer sync background task
    let sync_pool = pool.clone();
    let sync_config = config.indexer.clone();
    let sync_indexer_url = indexer_url.clone();
    let sync_lock_for_loop = sync_lock.clone();
    let reindex_notify_for_loop = reindex_notify.clone();
    tokio::spawn(async move {
        sync::indexer::run_sync_loop(
            sync_pool,
            sync_config,
            sync_indexer_url,
            sync_lock_for_loop,
            reindex_notify_for_loop,
        )
        .await;
    });

    let state = Arc::new(AppState {
        pool,
        jwt_secret,
        sync_lock,
        reindex_notify,
    });

    let base_path = config.server.base_path.clone();
    tracing::info!("Using base path: {base_path}");

    // Build router: API routes + static file serving for the frontend.
    // ServeDir serves real static files. not_found_service uses ServeFile to
    // serve index.html for any unmatched path (SPA client-side routes), so the
    // SPA boots and React Router renders the correct page.
    // not_found_service is required here (not router-level fallback) because
    // nest_service hands off to ServeDir internally — 404s from ServeDir never
    // bubble back up to the router.
    // When base_path is "/" serve everything from root (no prefix).
    // Otherwise nest_service strips the prefix before handing off to ServeDir,
    // so /ootle/community-templates/assets/foo.css resolves to static/assets/foo.css.
    // Use fallback() not not_found_service(): the latter wraps with SetStatus(404)
    // which breaks SPA deep-link navigation. fallback() preserves ServeFile's
    // natural 200 response for index.html.
    let serve_dir = ServeDir::new("static").fallback(ServeFile::new("static/index.html"));
    let api_router = api::router(state, &config.server.base_path).layer(
        TraceLayer::new_for_http()
            .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
            .on_response(DefaultOnResponse::new().level(Level::INFO)),
    );
    let app = if config.server.base_path == "/" {
        api_router.fallback_service(serve_dir)
    } else {
        api_router.nest_service(&config.server.base_path, serve_dir)
    }
    .layer(axum::middleware::from_fn(security_headers_middleware));

    let bind = format!("{}:{}", config.server.bind_address, config.server.port);
    tracing::info!("Starting server on {bind}");
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn security_headers_middleware(
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let mut resp = next.run(req).await;
    let headers = resp.headers_mut();
    headers.insert(
        axum::http::header::CONTENT_SECURITY_POLICY,
        axum::http::HeaderValue::from_static(
            "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' https: data:; connect-src 'self'; font-src 'self'; frame-ancestors 'none'; base-uri 'self'; form-action 'self'",
        ),
    );
    headers.insert(
        axum::http::header::X_CONTENT_TYPE_OPTIONS,
        axum::http::HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        axum::http::header::X_FRAME_OPTIONS,
        axum::http::HeaderValue::from_static("DENY"),
    );
    resp
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::get, Router};

    #[tokio::test]
    async fn security_headers_are_attached_to_responses() {
        let app = Router::new()
            .route("/health", get(|| async { "ok" }))
            .layer(axum::middleware::from_fn(security_headers_middleware));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind listener");
        let addr = listener.local_addr().expect("local addr");
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve");
        });

        let resp = reqwest::get(format!("http://{addr}/health"))
            .await
            .expect("request health");
        let headers = resp.headers();

        let csp = headers
            .get(axum::http::header::CONTENT_SECURITY_POLICY)
            .and_then(|v| v.to_str().ok())
            .expect("Content-Security-Policy header");
        assert!(csp.contains("default-src 'self'"));
        assert!(csp.contains("script-src 'self'"));
        assert!(csp.contains("frame-ancestors 'none'"));

        assert_eq!(
            headers
                .get(axum::http::header::X_CONTENT_TYPE_OPTIONS)
                .and_then(|v| v.to_str().ok()),
            Some("nosniff")
        );
        assert_eq!(
            headers
                .get(axum::http::header::X_FRAME_OPTIONS)
                .and_then(|v| v.to_str().ok()),
            Some("DENY")
        );
    }
}
