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
            Self::NoTokenProvided | Self::ExpiredToken | Self::InvalidToken => {
                StatusCode::UNAUTHORIZED
            }
            Self::UnknownUser => StatusCode::NOT_FOUND,
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        // --- LOGS AUTOMATIQUES SELON LA CRITICITÉ ---
        match self {
            // NoTokenProvided : comportement courant (visiteur non connecté sur une route protégée) -> Pas de log lourd
            // ExpiredToken : normal en fin de session -> Pas besoin de spammer les logs d'erreurs
            // UnknownUser : token valide mais l'utilisateur a été supprimé entre-temps
            Self::NoTokenProvided | Self::ExpiredToken | Self::UnknownUser => {}
            Self::InvalidToken => {
                eprintln!("[AVERTISSEMENT SÉCURITÉ] Tentative d'accès avec un jeton JWT altéré ou invalide.");
            }
            Self::DatabaseError => {
                eprintln!("[ERREUR CRITIQUE JWT] Échec de la base de données lors de la vérification de l'utilisateur.");
            }
        }

        // --- GÉNÉRATION DE LA RÉPONSE HTTP ---
        match self {
            Self::NoTokenProvided | Self::ExpiredToken | Self::InvalidToken => {
                HttpResponse::Unauthorized().body(self.to_string())
            }
            Self::UnknownUser => HttpResponse::NotFound().body(self.to_string()),
            Self::DatabaseError => HttpResponse::InternalServerError().body(self.to_string()),
        }
    }
}
