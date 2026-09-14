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
            Self::Database(err) => err.error_response(),
            Self::Redis(err) => err.error_response(),
            Self::Jwt(err) => err.error_response(),
            Self::Email(_) => {
                HttpResponse::InternalServerError().body("Échec de l'envoi de l'e-mail")
            }
            Self::Serialization(err) => {
                HttpResponse::BadRequest().body(format!("Format JSON invalide : {err}"))
            }
        }
    }
}
