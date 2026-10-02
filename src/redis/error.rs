use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RedisError {
    #[error("Erreur de pool Redis : {0}")]
    Pool(String),
    #[error("Erreur du driver Redis : {0}")]
    Driver(String),
    #[error("Erreur interne Redis : {0}")]
    Internal(String),
    #[error("Erreur de valeur Redis : {0}")]
    Value(String),
}

impl ResponseError for RedisError {
    // 1. Définition explicite du code HTTP 500 pour les problèmes d'infrastructure cache
    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    // 2. Génération de la réponse avec logs automatiques selon la nature de l'erreur
    fn error_response(&self) -> HttpResponse {
        match self {
            Self::Pool(msg) => {
                tracing::error!(error = %msg, "Redis connection pool failure");
            }
            Self::Driver(msg) => {
                tracing::error!(error = %msg, "Redis command failed");
            }
            Self::Internal(msg) => {
                tracing::error!(error = %msg, "Redis internal error");
            }
            Self::Value(msg) => {
                tracing::warn!(error = %msg, "Unexpected Redis value");
            }
        }

        // Generic body: the driver message is logged above, not sent to the client (MAIR-391).
        HttpResponse::InternalServerError().body("Internal cache error")
    }
}
