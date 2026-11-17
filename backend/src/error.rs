use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use utoipa::ToSchema;

pub type AppResult<T> = Result<T, AppError>;

/// Reasons a two-factor verification can fail. Each maps to a stable error code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TwoFactorError {
    ChallengeNotFound,
    InvalidCode,
    Expired,
    AlreadyUsed,
    TooManyAttempts,
}

impl TwoFactorError {
    fn code(self) -> &'static str {
        match self {
            Self::ChallengeNotFound => "INVALID_CHALLENGE",
            Self::InvalidCode => "INVALID_CODE",
            Self::Expired => "CODE_EXPIRED",
            Self::AlreadyUsed => "CODE_ALREADY_USED",
            Self::TooManyAttempts => "TOO_MANY_ATTEMPTS",
        }
    }

    fn message(self) -> &'static str {
        match self {
            Self::ChallengeNotFound => "Login challenge not found. Start the login again.",
            Self::InvalidCode => "The verification code is incorrect.",
            Self::Expired => "The verification code has expired. Start the login again.",
            Self::AlreadyUsed => {
                "This verification code has already been used or was replaced by a newer login."
            }
            Self::TooManyAttempts => "Too many incorrect attempts. Start the login again.",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Validation(String),
    #[error("Invalid email or password.")]
    InvalidCredentials,
    #[error("{}", .0.message())]
    TwoFactor(TwoFactorError),
    #[error("{0}")]
    Unauthorized(String),
    #[error("{0}")]
    Forbidden(String),
    #[error("{0}")]
    NotFound(String),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("{0}")]
    Internal(String),
}

/// Uniform error body: `{"error": {"code": "...", "message": "..."}}`.
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorBody {
    pub error: ErrorDetail,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
}

impl AppError {
    pub fn status_and_code(&self) -> (StatusCode, &'static str) {
        match self {
            Self::BadRequest(_) => (StatusCode::BAD_REQUEST, "BAD_REQUEST"),
            Self::Validation(_) => (StatusCode::UNPROCESSABLE_ENTITY, "VALIDATION_ERROR"),
            Self::InvalidCredentials => (StatusCode::UNAUTHORIZED, "INVALID_CREDENTIALS"),
            Self::TwoFactor(e) => (StatusCode::UNAUTHORIZED, e.code()),
            Self::Unauthorized(_) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED"),
            Self::Forbidden(_) => (StatusCode::FORBIDDEN, "FORBIDDEN"),
            Self::NotFound(_) => (StatusCode::NOT_FOUND, "NOT_FOUND"),
            Self::Database(_) | Self::Internal(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR")
            }
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code) = self.status_and_code();
        let message = match &self {
            // Never leak internal details to clients; log them instead.
            Self::Database(e) => {
                tracing::error!(error = %e, "database error");
                "An unexpected error occurred.".to_string()
            }
            Self::Internal(e) => {
                tracing::error!(error = %e, "internal error");
                "An unexpected error occurred.".to_string()
            }
            other => other.to_string(),
        };
        let body = ErrorBody {
            error: ErrorDetail {
                code: code.to_string(),
                message,
            },
        };
        (status, Json(body)).into_response()
    }
}
