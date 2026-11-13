//! Uniform API errors.
//!
//! Note the deliberate absence of a "forbidden, this screen exists but is not
//! yours" case for screens: a non-member gets [`AppError::NotFound`], because a
//! 403 would confirm that a slug names a real screen.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("not found")]
    NotFound,

    #[error("authentication required")]
    Unauthorized,

    #[error("insufficient permissions")]
    Forbidden,

    #[error("{0}")]
    BadRequest(String),

    #[error("{0}")]
    Conflict(String),

    #[error("{0}")]
    Payload(String),

    #[error(transparent)]
    Database(#[from] sqlx::Error),

    #[error("{0}")]
    Internal(String),
}

impl AppError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::BadRequest(message.into())
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict(message.into())
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal(message.into())
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::Payload(_) => StatusCode::PAYLOAD_TOO_LARGE,
            Self::Database(_) | Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Stable machine-readable code so the client can branch without parsing
    /// prose.
    fn code(&self) -> &'static str {
        match self {
            Self::NotFound => "not_found",
            Self::Unauthorized => "unauthorized",
            Self::Forbidden => "forbidden",
            Self::BadRequest(_) => "bad_request",
            Self::Conflict(_) => "conflict",
            Self::Payload(_) => "payload_too_large",
            Self::Database(_) | Self::Internal(_) => "internal",
        }
    }
}

#[derive(Serialize)]
struct ErrorBody {
    error: &'static str,
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status();
        let code = self.code();

        // Internal failures are logged in full and reported as a generic
        // message, so database detail never reaches a client.
        let message = match &self {
            AppError::Database(e) => {
                tracing::error!(error = %e, "database error");
                "an internal error occurred".to_string()
            }
            AppError::Internal(detail) => {
                tracing::error!(detail = %detail, "internal error");
                "an internal error occurred".to_string()
            }
            other => other.to_string(),
        };

        (status, Json(ErrorBody { error: code, message })).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;

/// Maps a unique-violation into a caller-facing conflict, leaving other
/// database errors alone.
pub fn on_unique_violation(err: sqlx::Error, message: &str) -> AppError {
    match &err {
        sqlx::Error::Database(db) if db.is_unique_violation() => AppError::conflict(message),
        _ => AppError::Database(err),
    }
}
