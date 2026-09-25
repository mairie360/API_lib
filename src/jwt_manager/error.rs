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
    /// The token's session was revoked (logout, revocation, archived account...).
    #[error("Revoked token")]
    RevokedToken,
    /// The revocation list (Redis) could not be checked: the token is refused (fail closed).
    #[error("Token revocation check unavailable")]
    RevocationCheckUnavailable,
}

impl ResponseError for JWTCheckError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::NoTokenProvided
            | Self::ExpiredToken
            | Self::InvalidToken
            | Self::RevokedToken => StatusCode::UNAUTHORIZED,
            Self::UnknownUser => StatusCode::NOT_FOUND,
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            Self::RevocationCheckUnavailable => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    fn error_response(&self) -> HttpResponse {
        // --- LOGS AUTOMATIQUES SELON LA CRITICITÉ ---
        match self {
            // NoTokenProvided : comportement courant (visiteur non connecté sur une route protégée) -> Pas de log lourd
            // ExpiredToken : normal en fin de session -> Pas besoin de spammer les logs d'erreurs
            // UnknownUser : token valide mais l'utilisateur a été supprimé entre-temps
            Self::NoTokenProvided | Self::ExpiredToken | Self::UnknownUser | Self::RevokedToken => {
            }
            Self::InvalidToken => {
                eprintln!("[AVERTISSEMENT SÉCURITÉ] Tentative d'accès avec un jeton JWT altéré ou invalide.");
            }
            Self::DatabaseError => {
                eprintln!("[ERREUR CRITIQUE JWT] Échec de la base de données lors de la vérification de l'utilisateur.");
            }
            Self::RevocationCheckUnavailable => {
                eprintln!("[CRITICAL JWT] Redis unavailable: cannot check the token revocation list, token refused.");
            }
        }

        // --- GÉNÉRATION DE LA RÉPONSE HTTP ---
        match self {
            Self::NoTokenProvided
            | Self::ExpiredToken
            | Self::InvalidToken
            | Self::RevokedToken => HttpResponse::Unauthorized().body(self.to_string()),
            Self::UnknownUser => HttpResponse::NotFound().body(self.to_string()),
            Self::DatabaseError => HttpResponse::InternalServerError().body(self.to_string()),
            Self::RevocationCheckUnavailable => {
                HttpResponse::ServiceUnavailable().body(self.to_string())
            }
        }
    }
}
