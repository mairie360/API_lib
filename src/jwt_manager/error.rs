// Fichier : src/jwt_manager/error.rs

use actix_web::{
    http::{header::ContentType, StatusCode},
    HttpResponse, ResponseError,
};
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
    /// A newer token of the same session was issued (refresh): this one is no longer the active token
    /// of its session (MAIR-772).
    #[error("Replaced token")]
    ReplacedToken,
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
            | Self::RevokedToken
            | Self::ReplacedToken => StatusCode::UNAUTHORIZED,
            Self::UnknownUser => StatusCode::NOT_FOUND,
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            Self::RevocationCheckUnavailable => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    fn error_response(&self) -> HttpResponse {
        // --- Logs, by severity ---
        match self {
            // NoTokenProvided: usual case (anonymous visitor on a protected route), no log.
            // ExpiredToken: normal at the end of a session, no need to flood the error logs.
            // UnknownUser: valid token, but the user was deleted in the meantime.
            // RevokedToken: the user logged out or the session was revoked, expected.
            Self::NoTokenProvided
            | Self::ExpiredToken
            | Self::UnknownUser
            | Self::RevokedToken
            | Self::ReplacedToken => {}
            Self::InvalidToken => {
                tracing::warn!("Access attempted with a forged or invalid JWT.");
            }
            Self::DatabaseError => {
                tracing::error!("Database failure while checking the token's user.");
            }
            Self::RevocationCheckUnavailable => {
                tracing::error!(
                    "Redis unavailable: cannot check the token revocation list, token refused."
                );
            }
        }

        // --- HTTP response ---
        HttpResponse::build(self.status_code())
            .content_type(ContentType::plaintext())
            .body(self.to_string())
    }
}
