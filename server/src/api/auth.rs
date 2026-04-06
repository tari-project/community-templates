use std::sync::Arc;

use argon2::{
    password_hash::rand_core::OsRng, password_hash::SaltString, Argon2, PasswordHash,
    PasswordHasher, PasswordVerifier,
};
use axum::{
    body::Body, extract::State, http::Request, middleware::Next, response::Response, routing::post,
    Json, Router,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use super::AppState;
use crate::{db, error::AppError};

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}

pub fn login_routes() -> Router<Arc<AppState>> {
    Router::new().route("/auth/login", post(login))
}

async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    let admin = db::admins::get_by_username(&state.pool, &req.username)
        .await?
        .ok_or_else(|| AppError::unauthorized("Invalid credentials"))?;

    let parsed_hash = PasswordHash::new(&admin.password_hash)
        .map_err(|_| AppError::internal("Invalid stored password hash"))?;

    Argon2::default()
        .verify_password(req.password.as_bytes(), &parsed_hash)
        .map_err(|_| AppError::unauthorized("Invalid credentials"))?;

    let exp = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::hours(24))
        .expect("valid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        sub: admin.username,
        exp,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(&state.jwt_secret),
    )
    .map_err(|e| AppError::internal(format!("Failed to create token: {e}")))?;

    Ok(Json(LoginResponse { token }))
}

pub async fn jwt_middleware(
    State(state): State<Arc<AppState>>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::unauthorized("Missing Authorization header"))?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| AppError::unauthorized("Invalid Authorization header format"))?;

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(&state.jwt_secret),
        &Validation::default(),
    )
    .map_err(|_| AppError::unauthorized("Invalid or expired token"))?;

    req.extensions_mut().insert(token_data.claims);
    Ok(next.run(req).await)
}

pub fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::internal(format!("Failed to hash password: {e}")))?;
    Ok(hash.to_string())
}
