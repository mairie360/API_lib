use super::check_jwt_validity::resolve_session_user;
use super::error::JWTCheckError;
use super::session_keys::session_public_keys;
use crate::smart_db::SmartDatabase;
use jsonwebtoken::{decode_header, Algorithm, Header};

/// Whether a token with this header was signed by `Core_API`: an `EdDSA` token whose `kid` is
/// one of the keys of `JWT_PUBLIC_KEY`.
fn is_core_token(header: &Header) -> bool {
    header.alg == Algorithm::EdDSA
        && header.kid.as_deref().is_some_and(|kid| {
            session_public_keys()
                .unwrap_or_default()
                .iter()
                .any(|key| key.kid() == kid)
        })
}

/// Authenticates a bearer token and returns the `users.id` it designates.
///
/// Only session JWTs issued by `Core_API` are accepted: an `EdDSA` token with the `kid` of a key
/// of `JWT_PUBLIC_KEY`. They are checked like
/// [`super::check_jwt_validity`] (signature, `iss`/`aud` for `EdDSA`, expiry, revocation,
/// existing user; see [`super::decode_jwt`]). A token signed with any other algorithm or key
/// (an HS256 token, an RS256 token of an identity provider) is refused.
///
/// # Errors
///
/// - [`JWTCheckError::NoTokenProvided`] if the token is empty;
/// - [`JWTCheckError::ExpiredToken`] if the token is expired;
/// - [`JWTCheckError::InvalidToken`] if the token is unreadable, badly signed, not an access
///   token, or not signed by `Core_API`;
/// - [`JWTCheckError::RevokedToken`] if the token's session (`sid` claim) was revoked;
/// - [`JWTCheckError::RevocationCheckUnavailable`] if the token has a `sid` and Redis cannot be
///   queried (fail closed);
/// - [`JWTCheckError::DatabaseError`] if the lookup in the database fails;
/// - [`JWTCheckError::UnknownUser`] if no active account matches the token.
pub async fn authenticate_token(
    jwt: &str,
    db_interface: &SmartDatabase,
) -> Result<u64, JWTCheckError> {
    if jwt.is_empty() {
        tracing::debug!("No JWT token provided.");
        return Err(JWTCheckError::NoTokenProvided);
    }

    let header = decode_header(jwt).map_err(|err| {
        tracing::warn!(error = ?err, "JWT header decode error");
        JWTCheckError::InvalidToken
    })?;
    if !is_core_token(&header) {
        tracing::warn!("Token not signed by Core_API refused.");
        return Err(JWTCheckError::InvalidToken);
    }
    resolve_session_user(jwt, db_interface).await
}
