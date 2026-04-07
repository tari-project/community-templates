pub mod admin;
pub mod auth;
pub mod metadata;
pub mod search;
pub mod templates;

use std::sync::Arc;

use axum::{http::Method, middleware, Router};
use sqlx::SqlitePool;
use tower_http::cors::{Any, CorsLayer};

pub struct AppState {
    pub pool: SqlitePool,
    /// Resolved JWT secret bytes (random if not configured, stable for the lifetime of the process).
    pub jwt_secret: Vec<u8>,
}

pub fn router(state: Arc<AppState>) -> Router {
    let public = Router::new()
        .merge(templates::routes())
        .merge(search::routes())
        .merge(auth::login_routes());

    // Metadata submission needs CORS for cross-origin CLI/browser submissions
    let metadata_routes = Router::new().merge(metadata::routes()).layer(
        CorsLayer::new()
            .allow_methods([Method::POST])
            .allow_origin(Any)
            .allow_headers(Any),
    );

    let admin = Router::new()
        .merge(admin::routes())
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::jwt_middleware,
        ));

    Router::new()
        .nest("/api", public)
        .nest("/api", metadata_routes)
        .nest("/api/admin", admin)
        .with_state(state)
}
