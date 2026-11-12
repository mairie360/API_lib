use super::check_jwt_validity::resolve_legacy_user;
use super::error::JWTCheckError;
use super::session_keys::session_public_keys;
use crate::database::db_interface::id_from_sql;
use crate::database::error::DbError;
use crate::database::query_views::GetUserIdByEmailQueryView;
use crate::error::ApiLibError;
use crate::keycloak::KeycloakTokenVerifier;
use crate::smart_db::SmartDatabase;
use jsonwebtoken::{decode_header, Algorithm, AlgorithmFamily, Header};

/// Whether a token with this header was signed by `Core_API`: a historical HMAC token, or an
/// `EdDSA` token whose `kid` is one of the keys of `JWT_PUBLIC_KEY` (MAIR-680). Keycloak tokens are
/// signed with the realm's keys, whose `kid` never matches.
fn is_core_token(header: &Header) -> bool {
    if header.alg.family() == AlgorithmFamily::Hmac {
        return true;
    }
    header.alg == Algorithm::EdDSA
        && header.kid.as_deref().is_some_and(|kid| {
            session_public_keys()
                .unwrap_or_default()
                .iter()
                .any(|key| key.kid() == kid)
        })
}

/// Authenticates a bearer token of either kind and returns the `users.id` it designates.
///
/// The kind is read from the token's `alg` header:
/// - an HMAC algorithm, or `EdDSA` with the `kid` of a key of `JWT_PUBLIC_KEY`, means a session
///   JWT issued by `Core_API`: it is checked like [`super::check_jwt_validity`] (signature,
///   `iss`/`aud` for `EdDSA`, expiry, revocation, existing user; see [`super::decode_jwt`]);
/// - any other algorithm means a Keycloak access token: it is verified by `keycloak` (see
///   [`KeycloakTokenVerifier::verify`]) and its verified e-mail is matched, case-insensitively,
///   to exactly one active account (ambiguous matches are refused, see
///   [`GetUserIdByEmailQueryView`]). Without a verifier (Keycloak not configured) such a token is
///   invalid.
///
/// # Errors
///
/// - [`JWTCheckError::NoTokenProvided`] if the token is empty;
/// - [`JWTCheckError::ExpiredToken`] if the token is expired;
/// - [`JWTCheckError::InvalidToken`] if the token is unreadable, badly signed, not an access
///   token, issued for another realm or audience, or a Keycloak token while Keycloak is disabled;
/// - [`JWTCheckError::RevokedToken`] if a historical token's session (`sid` claim) was revoked;
/// - [`JWTCheckError::RevocationCheckUnavailable`] if a historical token has a `sid` and Redis
///   cannot be queried (fail closed);
/// - [`JWTCheckError::EmailNotVerified`] if a Keycloak token carries no verified e-mail;
/// - [`JWTCheckError::IdentityProviderUnavailable`] if the realm keys cannot be fetched;
/// - [`JWTCheckError::DatabaseError`] if the lookup in the database fails;
/// - [`JWTCheckError::UnknownUser`] if no active account, or more than one, matches the token.
pub async fn authenticate_token(
    jwt: &str,
    db_interface: &SmartDatabase,
    keycloak: Option<&KeycloakTokenVerifier>,
) -> Result<u64, JWTCheckError> {
    if jwt.is_empty() {
        tracing::debug!("No JWT token provided.");
        return Err(JWTCheckError::NoTokenProvided);
    }

    let header = decode_header(jwt).map_err(|err| {
        tracing::warn!(error = ?err, "JWT header decode error");
        JWTCheckError::InvalidToken
    })?;
    if is_core_token(&header) {
        return resolve_legacy_user(jwt, db_interface).await;
    }

    let verifier = keycloak.ok_or_else(|| {
        tracing::warn!("Keycloak token received but Keycloak is not configured on this API.");
        JWTCheckError::InvalidToken
    })?;
    let identity = verifier.verify(jwt).await?;

    let query_view = GetUserIdByEmailQueryView::new(&identity.email);
    match db_interface.fetch_scalar::<i32, _>(&query_view).await {
        Ok(user_id) => Ok(id_from_sql(user_id)),
        Err(ApiLibError::Database(DbError::NotFound)) => {
            // The Keycloak subject identifies the identity; the e-mail is personal data and stays
            // out of the logs (MAIR-290).
            tracing::info!(
                subject = %identity.subject,
                "No single active account matches the Keycloak identity."
            );
            Err(JWTCheckError::UnknownUser)
        }
        Err(e) => {
            tracing::error!(error = %e, "Database query error");
            Err(JWTCheckError::DatabaseError)
        }
    }
}
