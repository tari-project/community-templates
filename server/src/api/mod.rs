pub mod admin;
pub mod auth;
pub mod metadata;
pub mod search;
pub mod templates;

use std::sync::Arc;

use axum::{middleware, Router};
use sqlx::PgPool;
use tower_http::cors::CorsLayer;

pub struct AppState {
    pub pool: PgPool,
    /// Resolved JWT secret bytes (random if not configured, stable for the lifetime of the process).
    pub jwt_secret: Vec<u8>,
}

pub fn router(state: Arc<AppState>) -> Router {
    let public = Router::new()
        .merge(templates::routes())
        .merge(search::routes())
        .merge(metadata::routes())
        .merge(auth::login_routes());

    let admin = Router::new()
        .merge(admin::routes())
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::jwt_middleware,
        ));

    Router::new()
        .nest("/api", public)
        .nest("/api/admin", admin)
        .layer(CorsLayer::permissive())
        .with_state(state)
}
