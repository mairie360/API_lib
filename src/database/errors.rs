use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Failed to connect to database: {0}")]
    ConnectionFailed(String),

    #[error("Database connection is closed")]
    ConnectionClosed,

    #[error("The database client is not initialized or has been dropped")]
    NotInitialized,

    #[error("Raw database driver error: {0}")]
    DriverError(#[from] tokio_postgres::Error),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Execution timeout")]
    Timeout,

    #[error("Internal library error: {0}")]
    Internal(String),
}
