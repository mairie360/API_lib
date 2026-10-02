use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Row does not match the DTO: {0}")]
    MappingError(String),

    #[error("Unique constraint violation: {0}")]
    UniqueViolation(String),

    #[error("Foreign key violation: {0}")]
    ForeignKeyViolation(String),

    #[error("Resource not found")]
    NotFound,

    #[error("Database error: {0}")]
    Sqlx(sqlx::Error),
}

impl From<sqlx::Error> for DbError {
    fn from(err: sqlx::Error) -> Self {
        match &err {
            sqlx::Error::RowNotFound => Self::NotFound,
            sqlx::Error::Database(db_err) => {
                if let Some(code) = db_err.code() {
                    match code.as_ref() {
                        "23505" => return Self::UniqueViolation(db_err.message().to_string()),
                        "23503" => return Self::ForeignKeyViolation(db_err.message().to_string()),
                        _ => {}
                    }
                }
                Self::Sqlx(err)
            }
            _ => Self::Sqlx(err),
        }
    }
}

impl DbError {
    /// Logs the error at its severity: nothing for `NotFound` (a normal client outcome), a
    /// warning for constraint violations, an error for everything that ends in a `500`.
    pub fn log(&self) {
        match self {
            Self::NotFound => {}
            Self::UniqueViolation(msg) => {
                tracing::warn!(error = %msg, "Unique constraint violation");
            }
            Self::ForeignKeyViolation(msg) => {
                tracing::warn!(error = %msg, "Foreign key violation");
            }
            Self::MappingError(msg) => {
                tracing::error!(error = %msg, "Database row does not match the DTO");
            }
            Self::Internal(msg) => {
                tracing::error!(error = %msg, "Database internal error");
            }
            Self::Sqlx(err) => {
                tracing::error!(error = ?err, "Database driver error");
            }
        }
    }
}

impl ResponseError for DbError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::UniqueViolation(_) => StatusCode::CONFLICT,
            Self::ForeignKeyViolation(_) => StatusCode::BAD_REQUEST,
            Self::MappingError(_) | Self::Internal(_) | Self::Sqlx(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }

    fn error_response(&self) -> HttpResponse {
        self.log();

        // --- HTTP response: generic bodies only. Postgres messages name tables, columns and
        // constraints, and are logged above instead of being sent to the client (MAIR-391). ---
        let body = match self {
            Self::NotFound => "Resource not found",
            Self::UniqueViolation(_) => "Data conflict: the resource already exists",
            Self::ForeignKeyViolation(_) => "Invalid reference to another resource",
            Self::MappingError(_) | Self::Internal(_) | Self::Sqlx(_) => "Internal database error",
        };
        HttpResponse::build(self.status_code()).body(body)
    }
}
