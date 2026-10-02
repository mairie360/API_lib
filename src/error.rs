use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use thiserror::Error;

use crate::{
    database::error::DbError, jwt_manager::error::JWTCheckError, keycloak::KeycloakError,
    password::error::PasswordError, redis::error::RedisError,
};

#[derive(Debug, Error)]
pub enum ApiLibError {
    #[error(transparent)]
    Database(#[from] DbError),

    #[error(transparent)]
    Redis(#[from] RedisError),

    #[error(transparent)]
    Jwt(#[from] JWTCheckError),

    #[error(transparent)]
    Keycloak(#[from] KeycloakError),

    #[error(transparent)]
    Password(#[from] PasswordError),

    #[error(transparent)]
    Email(#[from] resend_rs::Error),

    #[error("JSON serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// What an [`ApiLibError`] means for the caller, so an API can map it to its own error enum by
/// type instead of turning every failure into the same status (MAIR-421).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// No row matched (`404`).
    NotFound,
    /// A unique constraint refused the write: the resource already exists (`409`).
    Conflict,
    /// A foreign key refused the write: the request references a missing resource (`400`).
    InvalidReference,
    /// Any other refusal caused by the request: bad token, bad JSON… (`4xx`).
    Client,
    /// A failure of the service or of a dependency (`5xx`): always worth an error log.
    Internal,
}

impl ErrorKind {
    /// Whether the failure comes from the request rather than from the service.
    #[must_use]
    pub const fn is_client_error(self) -> bool {
        !matches!(self, Self::Internal)
    }
}

impl ApiLibError {
    /// The kind of failure, derived from the HTTP status the lib would answer with.
    #[must_use]
    pub fn kind(&self) -> ErrorKind {
        match self {
            Self::Database(DbError::NotFound) => ErrorKind::NotFound,
            Self::Database(DbError::UniqueViolation(_)) => ErrorKind::Conflict,
            Self::Database(DbError::ForeignKeyViolation(_)) => ErrorKind::InvalidReference,
            _ if self.status_code().is_client_error() => ErrorKind::Client,
            _ => ErrorKind::Internal,
        }
    }
}

/// Logs `error` with the `context` of the caller (the operation that failed, e.g.
/// `"create chat"`) at the level its [`ErrorKind`] deserves, and gives it back.
///
/// Meant for the APIs that map lib errors to their own error enum, so a `500` always leaves a
/// cause in the logs:
///
/// ```ignore
/// let chat = smart_db
///     .fetch_one(&GetChatView::new(chat_id))
///     .await
///     .map_err(|e| match log_error("get chat", e).kind() {
///         ErrorKind::NotFound => ChatError::NotFound,
///         _ => ChatError::DatabaseError,
///     })?;
/// ```
///
/// An error returned as is (the lib's own `ResponseError`) is already logged by
/// `error_response`; do not log it twice.
pub fn log_error(context: &str, error: ApiLibError) -> ApiLibError {
    match error.kind() {
        ErrorKind::NotFound => tracing::debug!(context, error = %error, "not found"),
        ErrorKind::Conflict | ErrorKind::InvalidReference | ErrorKind::Client => {
            tracing::warn!(context, error = %error, "request refused");
        }
        ErrorKind::Internal => {
            tracing::error!(context, error = %error, details = ?error, "operation failed");
        }
    }
    error
}

impl ResponseError for ApiLibError {
    /// The status of the wrapped error, so `status_code()` agrees with `error_response()`.
    fn status_code(&self) -> StatusCode {
        match self {
            Self::Database(err) => err.status_code(),
            Self::Redis(err) => err.status_code(),
            Self::Jwt(err) => err.status_code(),
            Self::Keycloak(err) => err.status_code(),
            Self::Password(err) => err.status_code(),
            Self::Email(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Serialization(_) => StatusCode::BAD_REQUEST,
        }
    }

    fn error_response(&self) -> HttpResponse {
        match self {
            Self::Database(err) => err.error_response(),
            Self::Redis(err) => err.error_response(),
            Self::Jwt(err) => err.error_response(),
            Self::Keycloak(err) => err.error_response(),
            Self::Password(err) => err.error_response(),
            Self::Email(_) => HttpResponse::InternalServerError().body("Failed to send the e-mail"),
            Self::Serialization(err) => {
                HttpResponse::BadRequest().body(format!("Invalid JSON: {err}"))
            }
        }
    }
}
