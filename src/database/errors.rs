use crate::database::queries::QueryError;
use thiserror::Error;

#[derive(Clone, Debug, Error, PartialEq)]
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

impl From<sqlx::Error> for DatabaseError {
    fn from(err: sqlx::Error) -> Self {
        eprintln!("Database error log: {}", err);

        match err {
            // 1. Gestion spécifique de l'absence de ligne (Pour tes tests About/Login)
            sqlx::Error::RowNotFound => DatabaseError::Query(QueryError::NoResults),

            // 2. Analyse des erreurs renvoyées par Postgres
            sqlx::Error::Database(db_err) => {
                // Utilisation des codes d'erreur PostgreSQL (23505 = Unique Violation)
                if db_err.code().map(|c| c == "23505").unwrap_or(false) {
                    DatabaseError::Query(QueryError::ConstraintViolation(
                        db_err.message().to_string(),
                    ))
                } else {
                    DatabaseError::Query(QueryError::ExecutionFailed(db_err.message().to_string()))
                }
            }

            // 3. Gestion du pool et timeout
            sqlx::Error::PoolTimedOut => DatabaseError::Timeout,
            sqlx::Error::PoolClosed => DatabaseError::ConnectionClosed,

            // 4. Fallback pour le reste
            _ => DatabaseError::Internal(err.to_string()),
        }
    }
}
