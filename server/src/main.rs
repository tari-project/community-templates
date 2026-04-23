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
    trace::TraceLayer,
};

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

    // Start indexer sync background task
    let sync_pool = pool.clone();
    let sync_config = config.indexer.clone();
    let sync_indexer_url = indexer_url.clone();
    tokio::spawn(async move {
        sync::indexer::run_sync_loop(sync_pool, sync_config, sync_indexer_url).await;
    });

    let state = Arc::new(AppState { pool, jwt_secret });

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
    let app = if config.server.base_path == "/" {
        api::router(state, &config.server.base_path).fallback_service(serve_dir)
    } else {
        api::router(state, &config.server.base_path)
            .nest_service(&config.server.base_path, serve_dir)
    }
    .layer(TraceLayer::new_for_http());

    let bind = format!("{}:{}", config.server.bind_address, config.server.port);
    tracing::info!("Starting server on {bind}");
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
