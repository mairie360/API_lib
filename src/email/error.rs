// Fichier : src/email/error.rs

use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EmailError {
    #[error("Erreur du service Resend : {0}")]
    Resend(#[from] resend_rs::Error),
}

impl ResponseError for EmailError {
    // 1. Définition explicite du code HTTP 500 Internal Server Error
    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    // 2. Génération de la réponse HTTP avec log automatique de l'incident
    fn error_response(&self) -> HttpResponse {
        match self {
            EmailError::Resend(err) => {
                // Log critique indispensable pour tracer les pannes du service tiers
                eprintln!(
                    "[ERREUR CRITIQUE EMAIL] Échec du service Resend : {:?}",
                    err
                );
            }
        }

        HttpResponse::InternalServerError().body("Échec lors de l'envoi de l'e-mail")
    }
}
