use crate::database::queries::QueryError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum DatabaseError {
    #[error("Failed to connect to database: {0}")]
    ConnectionFailed(String),

    #[error("Database connection is closed")]
    ConnectionClosed,

    #[error("The database client is not initialized or has been dropped")]
    NotInitialized,

    #[error("Raw database driver error: {0}")]
    DriverError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Execution timeout")]
    Timeout,

    #[error("Internal library error: {0}")]
    Internal(String),

    #[error("Query error: {0}")]
    Query(#[from] QueryError),
}

/// Permet de convertir facilement les erreurs tokio_postgres en DatabaseError
impl From<tokio_postgres::Error> for DatabaseError {
    fn from(err: tokio_postgres::Error) -> Self {
        // On peut affiner ici en analysant err.code() si besoin
        let err_str = err.to_string();
        if err_str.contains("unique constraint") {
            DatabaseError::Query(QueryError::ConstraintViolation(err_str))
        } else if err_str.contains("0 rows") {
            DatabaseError::Query(QueryError::NoResults)
        } else {
            DatabaseError::Query(QueryError::ExecutionFailed(err_str))
        }
    }
}
