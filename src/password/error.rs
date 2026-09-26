use actix_web::{http::StatusCode, HttpResponse, ResponseError};
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
        eprintln!("[ERREUR CRITIQUE MOT DE PASSE] {self}");
        HttpResponse::InternalServerError().body(self.to_string())
    }
}
