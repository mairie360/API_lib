use actix_web::{HttpResponse, ResponseError};
use thiserror::Error;

use crate::{
    database::error::DbError, jwt_manager::error::JWTCheckError, redis::error::RedisError,
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
    Email(#[from] resend_rs::Error),

    #[error("Erreur de sérialisation JSON : {0}")]
    Serialization(#[from] serde_json::Error),
}

impl ResponseError for ApiLibError {
    fn error_response(&self) -> HttpResponse {
        match self {
            ApiLibError::Database(err) => err.error_response(),
            ApiLibError::Redis(err) => err.error_response(),
            ApiLibError::Jwt(err) => err.error_response(),
            ApiLibError::Email(_) => {
                HttpResponse::InternalServerError().body("Échec de l'envoi de l'e-mail")
            }
            ApiLibError::Serialization(err) => {
                HttpResponse::BadRequest().body(format!("Format JSON invalide : {}", err))
            }
        }
    }
}
