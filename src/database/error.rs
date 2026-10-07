use std::fmt::{self, Write as _};

use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use sqlx::postgres::PgDatabaseError;
use thiserror::Error;

/// Database failure, safe to log (MAIR-290).
///
/// Its `Display` and `Debug` describe the failure by its type and context (SQLSTATE,
/// constraint, table, column), never by the values involved. A Postgres message can carry them: `22P02` quotes the invalid input, a `RAISE` formats its
/// parameters, and the `DETAIL` of a check or not-null violation lists the whole failing row
/// (e-mail, phone, password hash…). The variants keep their shape, so the APIs can still match
/// on `DbError::Sqlx(sqlx::Error::Database(_))` and read the code; do not log that inner error
/// yourself.
#[derive(Error)]
pub enum DbError {
    #[error("Internal error: {0}")]
    Internal(String),

    /// The row could not be read as the DTO: the serde message, without the value (see
    /// [`describe_json_error`](crate::error::describe_json_error)).
    #[error("Row does not match the DTO: {0}")]
    MappingError(String),

    /// The name of the unique constraint that refused the write.
    #[error("Unique constraint violation: {0}")]
    UniqueViolation(String),

    /// The name of the foreign key that refused the write.
    #[error("Foreign key violation: {0}")]
    ForeignKeyViolation(String),

    #[error("Resource not found")]
    NotFound,

    #[error("Database error: {}", describe_sqlx_error(.0))]
    Sqlx(sqlx::Error),
}

impl fmt::Debug for DbError {
    // Same text as `Display`: a derived `Debug` would print the full `PgDatabaseError`, `DETAIL`
    // included, wherever the error is logged with `?`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let variant = match self {
            Self::Internal(_) => "Internal",
            Self::MappingError(_) => "MappingError",
            Self::UniqueViolation(_) => "UniqueViolation",
            Self::ForeignKeyViolation(_) => "ForeignKeyViolation",
            Self::NotFound => "NotFound",
            Self::Sqlx(_) => "Sqlx",
        };
        write!(f, "DbError::{variant}({self})")
    }
}

/// Describes a sqlx error without the values it may carry: for an error returned by Postgres,
/// its SQLSTATE and the constraint, table and column it names; for a decoding error, the column
/// index only.
#[must_use]
pub fn describe_sqlx_error(error: &sqlx::Error) -> String {
    match error {
        sqlx::Error::Database(db_error) => {
            let mut text = format!(
                "SQLSTATE {}",
                db_error.code().as_deref().unwrap_or("unknown")
            );
            let pg = db_error.try_downcast_ref::<PgDatabaseError>();
            let context = [
                ("constraint", db_error.constraint()),
                ("table", db_error.table()),
                ("column", pg.and_then(PgDatabaseError::column)),
            ];
            for (name, value) in context {
                if let Some(value) = value {
                    let _ = write!(text, ", {name} {value}");
                }
            }
            text
        }
        sqlx::Error::ColumnDecode { index, .. } => format!("cannot decode column {index}"),
        sqlx::Error::Decode(_) => "cannot decode a value".to_string(),
        other => other.to_string(),
    }
}

impl From<sqlx::Error> for DbError {
    fn from(err: sqlx::Error) -> Self {
        match &err {
            sqlx::Error::RowNotFound => Self::NotFound,
            sqlx::Error::Database(db_err) => {
                let constraint = || db_err.constraint().unwrap_or("unknown").to_string();
                if let Some(code) = db_err.code() {
                    match code.as_ref() {
                        "23505" => return Self::UniqueViolation(constraint()),
                        "23503" => return Self::ForeignKeyViolation(constraint()),
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
            Self::UniqueViolation(constraint) => {
                tracing::warn!(constraint = %constraint, "Unique constraint violation");
            }
            Self::ForeignKeyViolation(constraint) => {
                tracing::warn!(constraint = %constraint, "Foreign key violation");
            }
            Self::MappingError(msg) => {
                tracing::error!(error = %msg, "Database row does not match the DTO");
            }
            Self::Internal(msg) => {
                tracing::error!(error = %msg, "Database internal error");
            }
            Self::Sqlx(err) => {
                tracing::error!(error = %describe_sqlx_error(err), "Database driver error");
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
        // constraints; the logs above carry the SQLSTATE and the constraint instead (MAIR-391). ---
        let body = match self {
            Self::NotFound => "Resource not found",
            Self::UniqueViolation(_) => "Data conflict: the resource already exists",
            Self::ForeignKeyViolation(_) => "Invalid reference to another resource",
            Self::MappingError(_) | Self::Internal(_) | Self::Sqlx(_) => "Internal database error",
        };
        HttpResponse::build(self.status_code()).body(body)
    }
}
