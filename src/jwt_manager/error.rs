// Fichier : src/jwt_manager/error.rs

use crate::keycloak::KeycloakError;
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
    /// A Keycloak token whose e-mail is missing or not verified by the realm.
    #[error("The Keycloak account has no verified e-mail address")]
    EmailNotVerified,
    /// The Keycloak realm keys could not be fetched to verify a token.
    #[error("The identity provider is unavailable")]
    IdentityProviderUnavailable,
}

impl From<KeycloakError> for JWTCheckError {
    fn from(error: KeycloakError) -> Self {
        match error {
            KeycloakError::InvalidToken => Self::InvalidToken,
            KeycloakError::ExpiredToken => Self::ExpiredToken,
            KeycloakError::EmailNotVerified => Self::EmailNotVerified,
            KeycloakError::Unavailable => Self::IdentityProviderUnavailable,
        }
    }
}

impl ResponseError for JWTCheckError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::NoTokenProvided
            | Self::ExpiredToken
            | Self::InvalidToken
            | Self::RevokedToken => StatusCode::UNAUTHORIZED,
            Self::NoTokenProvided | Self::ExpiredToken | Self::InvalidToken => {
                StatusCode::UNAUTHORIZED
            }
            Self::EmailNotVerified => StatusCode::FORBIDDEN,
            Self::UnknownUser => StatusCode::NOT_FOUND,
            Self::IdentityProviderUnavailable => StatusCode::BAD_GATEWAY,
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
            // EmailNotVerified : configuration du realm Keycloak, déjà tracée à la vérification
            Self::NoTokenProvided
            | Self::ExpiredToken
            | Self::UnknownUser
            | Self::EmailNotVerified => {}
            Self::InvalidToken => {
                eprintln!("[AVERTISSEMENT SÉCURITÉ] Tentative d'accès avec un jeton JWT altéré ou invalide.");
            }
            Self::IdentityProviderUnavailable => {
                eprintln!("[ERREUR CRITIQUE KEYCLOAK] Impossible de récupérer les clés du realm pour vérifier le jeton.");
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
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}
