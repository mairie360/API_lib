// Fichier : src/jwt_manager/error.rs

use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum JWTCheckError {
    #[error("Erreur de base de données")]
    DatabaseError,
    #[error("Aucun jeton fourni")]
    NoTokenProvided,
    #[error("Jeton expiré")]
    ExpiredToken,
    #[error("Jeton invalide")]
    InvalidToken,
    #[error("Utilisateur inconnu")]
    UnknownUser,
}

impl ResponseError for JWTCheckError {
    fn status_code(&self) -> StatusCode {
        match self {
            JWTCheckError::NoTokenProvided
            | JWTCheckError::ExpiredToken
            | JWTCheckError::InvalidToken => StatusCode::UNAUTHORIZED,
            JWTCheckError::UnknownUser => StatusCode::NOT_FOUND,
            JWTCheckError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        // --- LOGS AUTOMATIQUES SELON LA CRITICITÉ ---
        match self {
            JWTCheckError::NoTokenProvided => {
                // Comportement courant (visiteur non connecté sur une route protégée) -> Pas de log lourd
            }
            JWTCheckError::ExpiredToken => {
                // Normal en fin de session -> Pas besoin de spammer les logs d'erreurs
            }
            JWTCheckError::InvalidToken => {
                eprintln!("[AVERTISSEMENT SÉCURITÉ] Tentative d'accès avec un jeton JWT altéré ou invalide.");
            }
            JWTCheckError::UnknownUser => {
                // Token valide mais l'utilisateur a été supprimé entre-temps
            }
            JWTCheckError::DatabaseError => {
                eprintln!("[ERREUR CRITIQUE JWT] Échec de la base de données lors de la vérification de l'utilisateur.");
            }
        }

        // --- GÉNÉRATION DE LA RÉPONSE HTTP ---
        match self {
            JWTCheckError::NoTokenProvided
            | JWTCheckError::ExpiredToken
            | JWTCheckError::InvalidToken => HttpResponse::Unauthorized().body(self.to_string()),
            JWTCheckError::UnknownUser => HttpResponse::NotFound().body(self.to_string()),
            JWTCheckError::DatabaseError => {
                HttpResponse::InternalServerError().body(self.to_string())
            }
        }
    }
}
