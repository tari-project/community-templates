use std::sync::Arc;

use argon2::{password_hash::SaltString, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::{
    body::Body,
    extract::State,
    http::{header, HeaderMap, HeaderValue, Request},
    middleware::Next,
    response::Response,
    routing::post,
    Json, Router,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rand::RngCore;
use serde::{Deserialize, Serialize};

use super::AppState;
use crate::{db, error::AppError};

const ADMIN_COOKIE_NAME: &str = "admin_token";
const SESSION_MAX_AGE_SECS: i64 = 24 * 60 * 60;

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
    Router::new()
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
}

async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Result<(HeaderMap, Json<LoginResponse>), AppError> {
    let admin = db::admins::get_by_username(&state.pool, &req.username)
        .await?
        .ok_or_else(|| AppError::unauthorized("Invalid credentials"))?;

    let parsed_hash = PasswordHash::new(&admin.password_hash)
        .map_err(|_| AppError::internal("Invalid stored password hash"))?;

    Argon2::default()
        .verify_password(req.password.as_bytes(), &parsed_hash)
        .map_err(|_| AppError::unauthorized("Invalid credentials"))?;

    let exp = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::seconds(SESSION_MAX_AGE_SECS))
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

    let cookie = format!(
        "{ADMIN_COOKIE_NAME}={token}; Path=/; Max-Age={SESSION_MAX_AGE_SECS}; HttpOnly; SameSite=Strict"
    );
    let mut headers = HeaderMap::new();
    if let Ok(val) = HeaderValue::from_str(&cookie) {
        headers.insert(header::SET_COOKIE, val);
    }

    Ok((headers, Json(LoginResponse { token })))
}

async fn logout() -> (HeaderMap, Json<serde_json::Value>) {
    let clear_cookie =
        format!("{ADMIN_COOKIE_NAME}=; Path=/; Max-Age=0; HttpOnly; SameSite=Strict");
    let mut headers = HeaderMap::new();
    if let Ok(val) = HeaderValue::from_str(&clear_cookie) {
        headers.insert(header::SET_COOKIE, val);
    }
    (headers, Json(serde_json::json!({ "ok": true })))
}

fn extract_cookie_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|cookie_str| cookie_str.split(';'))
        .find_map(|pair| {
            let (k, v) = pair.trim().split_once('=')?;
            (k.trim() == ADMIN_COOKIE_NAME && !v.trim().is_empty()).then_some(v.trim())
        })
}

pub async fn jwt_middleware(
    State(state): State<Arc<AppState>>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    let token = if let Some(auth_header) = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    {
        auth_header
            .strip_prefix("Bearer ")
            .ok_or_else(|| AppError::unauthorized("Invalid Authorization header format"))?
    } else if let Some(cookie_token) = extract_cookie_token(req.headers()) {
        cookie_token
    } else {
        return Err(AppError::unauthorized("Missing Authorization header"));
    };

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
    // password-hash's transitive rand_core 0.6 OsRng is feature-gated behind getrandom,
    // which isn't activated in our resolved graph. Source entropy from rand 0.9 directly.
    let mut salt_bytes = [0u8; 16];
    rand::rng().fill_bytes(&mut salt_bytes);
    let salt = SaltString::encode_b64(&salt_bytes)
        .map_err(|e| AppError::internal(format!("Failed to encode salt: {e}")))?;
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::internal(format!("Failed to hash password: {e}")))?;
    Ok(hash.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_admin_token_from_cookie_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            HeaderValue::from_static("other=1; admin_token=eyJhbGciOiJIUzI1NiJ9.test; theme=dark"),
        );
        assert_eq!(
            extract_cookie_token(&headers),
            Some("eyJhbGciOiJIUzI1NiJ9.test")
        );

        let empty = HeaderMap::new();
        assert_eq!(extract_cookie_token(&empty), None);
    }

    #[tokio::test]
    async fn logout_clears_http_only_cookie() {
        let (headers, body) = logout().await;
        let set_cookie = headers
            .get(header::SET_COOKIE)
            .and_then(|v| v.to_str().ok())
            .expect("Set-Cookie header");
        assert!(set_cookie.contains("admin_token=;"));
        assert!(set_cookie.contains("Max-Age=0"));
        assert!(set_cookie.contains("HttpOnly"));
        assert!(set_cookie.contains("SameSite=Strict"));
        assert_eq!(body.0["ok"], serde_json::json!(true));
    }
}
