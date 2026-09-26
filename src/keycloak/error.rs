use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use thiserror::Error;

/// Failure while validating a Keycloak access token.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum KeycloakError {
    /// The token is malformed, badly signed, signed with an unknown or unexpected key, not an
    /// access token, or issued by another realm or for another client.
    #[error("Invalid Keycloak token")]
    InvalidToken,
    /// The token is well-formed and correctly signed but its `exp` claim is in the past.
    #[error("Expired Keycloak token")]
    ExpiredToken,
    /// The token carries no e-mail, or Keycloak has not verified it: it cannot be matched safely
    /// against a Mairie 360 account.
    #[error("The Keycloak account has no verified e-mail address")]
    EmailNotVerified,
    /// The realm's key set could not be fetched (Keycloak unreachable or answering unexpectedly).
    #[error("Keycloak is unavailable")]
    Unavailable,
}

impl ResponseError for KeycloakError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::InvalidToken | Self::ExpiredToken => StatusCode::UNAUTHORIZED,
            Self::EmailNotVerified => StatusCode::FORBIDDEN,
            Self::Unavailable => StatusCode::BAD_GATEWAY,
        }
    }

    fn error_response(&self) -> HttpResponse {
        match self {
            // An expired token is the normal end of a session, and an unverified e-mail is a
            // realm configuration matter: neither deserves an error log.
            Self::ExpiredToken | Self::EmailNotVerified => {}
            Self::InvalidToken => {
                eprintln!(
                    "[SECURITY WARNING] Access attempted with a forged or invalid Keycloak token."
                );
            }
            Self::Unavailable => {
                eprintln!("[CRITICAL KEYCLOAK ERROR] The realm key set could not be fetched.");
            }
        }
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}
