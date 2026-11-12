use actix_web::{
    http::{header::ContentType, StatusCode},
    HttpResponse, ResponseError,
};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PasswordError {
    #[error("Échec du hachage du mot de passe")]
    HashFailed,
    #[error("Empreinte de mot de passe invalide ou corrompue")]
    InvalidHash,
}

impl ResponseError for PasswordError {
    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn error_response(&self) -> HttpResponse {
        tracing::error!("Password hashing failure: {self}");
        HttpResponse::InternalServerError()
            .content_type(ContentType::plaintext())
            .body(self.to_string())
    }
}
