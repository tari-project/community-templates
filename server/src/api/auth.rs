use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};

use argon2::{password_hash::SaltString, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::Request,
    middleware::Next,
    response::Response,
    routing::post,
    Json, Router,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use super::AppState;
use crate::{db, error::AppError};

/// Maximum login attempts allowed per IP address within the sliding window.
const MAX_IP_ATTEMPTS: usize = 5;
/// Duration of the sliding rate limiting window for IP addresses (1 minute).
const IP_WINDOW: Duration = Duration::from_secs(60);

/// Maximum failed login attempts allowed per username before lockout.
const MAX_USER_FAILURES: usize = 5;
/// Duration of the lockout window for a specific username (5 minutes).
const USER_LOCKOUT_DURATION: Duration = Duration::from_secs(300);

#[derive(Default)]
pub struct LoginRateLimiter {
    /// Tracks recent attempt timestamps per IP address.
    ip_attempts: Mutex<HashMap<String, Vec<Instant>>>,
    /// Tracks (failure_count, lockout_until) per username.
    user_failures: Mutex<HashMap<String, (usize, Option<Instant>)>>,
}

impl LoginRateLimiter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Checks whether an attempt from the given IP and username is permitted.
    pub async fn check_rate_limit(&self, client_ip: &str, username: &str) -> Result<(), AppError> {
        let now = Instant::now();

        // 1. Check IP rate limit (sliding window)
        let mut ip_map = self.ip_attempts.lock().await;
        let attempts = ip_map.entry(client_ip.to_string()).or_default();
        attempts.retain(|&t| now.duration_since(t) < IP_WINDOW);

        if attempts.len() >= MAX_IP_ATTEMPTS {
            return Err(AppError::too_many_requests(
                "Too many login attempts from this IP address. Please wait a minute and try again.",
            ));
        }
        attempts.push(now);
        drop(ip_map);

        // 2. Check username lockout
        let mut user_map = self.user_failures.lock().await;
        if let Some((failures, lockout_until)) = user_map.get_mut(username) {
            if let Some(until) = *lockout_until {
                if now < until {
                    let remaining_secs = until.duration_since(now).as_secs() + 1;
                    return Err(AppError::too_many_requests(format!(
                        "Account is temporarily locked due to multiple failed login attempts. Please try again in {remaining_secs} seconds."
                    )));
                } else {
                    // Lockout period has elapsed; reset counter
                    *failures = 0;
                    *lockout_until = None;
                }
            }
        }

        Ok(())
    }

    /// Records a failed authentication attempt for a username, enforcing lockout if threshold reached.
    pub async fn record_failure(&self, username: &str) {
        let now = Instant::now();
        let mut user_map = self.user_failures.lock().await;
        let (failures, lockout_until) = user_map.entry(username.to_string()).or_insert((0, None));
        *failures += 1;
        if *failures >= MAX_USER_FAILURES {
            *lockout_until = Some(now + USER_LOCKOUT_DURATION);
        }
    }

    /// Resets the failure count for a username upon successful authentication.
    pub async fn record_success(&self, username: &str) {
        let mut user_map = self.user_failures.lock().await;
        user_map.remove(username);
    }
}

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
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    let client_ip = addr.ip().to_string();

    // 1. Enforce rate limiting before running expensive Argon2 password hashing (#33)
    state
        .login_rate_limiter
        .check_rate_limit(&client_ip, &req.username)
        .await?;

    let admin = match db::admins::get_by_username(&state.pool, &req.username).await? {
        Some(a) => a,
        None => {
            state.login_rate_limiter.record_failure(&req.username).await;
            return Err(AppError::unauthorized("Invalid credentials"));
        }
    };

    let parsed_hash = PasswordHash::new(&admin.password_hash)
        .map_err(|_| AppError::internal("Invalid stored password hash"))?;

    if let Err(_) = Argon2::default().verify_password(req.password.as_bytes(), &parsed_hash) {
        state.login_rate_limiter.record_failure(&req.username).await;
        return Err(AppError::unauthorized("Invalid credentials"));
    }

    // Authentication succeeded; clear any failure counts
    state.login_rate_limiter.record_success(&req.username).await;

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

    #[tokio::test]
    async fn test_login_rate_limiter_ip_throttling() {
        let limiter = LoginRateLimiter::new();
        let ip = "192.168.1.100";
        let user = "admin";

        // First 5 attempts should succeed
        for _ in 0..5 {
            assert!(limiter.check_rate_limit(ip, user).await.is_ok());
        }

        // 6th attempt from same IP must be rejected
        let res = limiter.check_rate_limit(ip, user).await;
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().status, axum::http::StatusCode::TOO_MANY_REQUESTS);

        // Different IP should still be allowed
        assert!(limiter.check_rate_limit("192.168.1.101", user).await.is_ok());
    }

    #[tokio::test]
    async fn test_login_rate_limiter_user_lockout() {
        let limiter = LoginRateLimiter::new();
        let user = "target_admin";

        // Record 4 failures
        for _ in 0..4 {
            limiter.record_failure(user).await;
        }
        assert!(limiter.check_rate_limit("10.0.0.1", user).await.is_ok());

        // 5th failure triggers lockout
        limiter.record_failure(user).await;
        let res = limiter.check_rate_limit("10.0.0.2", user).await;
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().status, axum::http::StatusCode::TOO_MANY_REQUESTS);

        // Success clears failures
        limiter.record_success(user).await;
        assert!(limiter.check_rate_limit("10.0.0.3", user).await.is_ok());
    }
}
