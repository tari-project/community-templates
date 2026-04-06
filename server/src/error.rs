use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub error: String,
}

#[derive(Debug)]
pub struct AppError {
    pub status: StatusCode,
    pub message: String,
}

impl AppError {
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: msg.into(),
        }
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: msg.into(),
        }
    }

    pub fn forbidden(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            message: msg.into(),
        }
    }

    pub fn unauthorized(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            message: msg.into(),
        }
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: msg.into(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorBody {
                error: self.message,
            }),
        )
            .into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        tracing::error!("Database error: {err}");
        Self::internal("Internal database error")
    }
}

/// Parse a template address string (with or without `template_` prefix) into a
/// `PublishedTemplateAddress`, returning an `AppError::bad_request` on failure.
pub fn parse_template_addr(
    s: &str,
) -> Result<tari_engine_types::published_template::PublishedTemplateAddress, AppError> {
    s.parse::<tari_engine_types::published_template::PublishedTemplateAddress>()
        .map_err(|e| AppError::bad_request(e.to_string()))
}
