use super::check_jwt_validity::resolve_legacy_user;
use super::error::JWTCheckError;
use crate::database::db_interface::id_from_sql;
use crate::database::error::DbError;
use crate::database::query_views::GetUserIdByEmailQueryView;
use crate::error::ApiLibError;
use crate::keycloak::KeycloakTokenVerifier;
use crate::smart_db::SmartDatabase;
use jsonwebtoken::{decode_header, AlgorithmFamily};

/// Authenticates a bearer token of either kind and returns the `users.id` it designates.
///
/// The kind is read from the token's `alg` header:
/// - an HMAC algorithm means a historical JWT issued by `Core_API`: it is checked like
///   [`super::check_jwt_validity`] (signature with `JWT_SECRET`, expiry, existing user);
/// - any other algorithm means a Keycloak access token: it is verified by `keycloak` (see
///   [`KeycloakTokenVerifier::verify`]) and its verified e-mail is matched, case-insensitively,
///   to an active account. Without a verifier (Keycloak not configured) such a token is invalid.
///
/// # Errors
///
/// - [`JWTCheckError::NoTokenProvided`] if the token is empty;
/// - [`JWTCheckError::ExpiredToken`] if the token is expired;
/// - [`JWTCheckError::InvalidToken`] if the token is unreadable, badly signed, not an access
///   token, issued for another realm or audience, or a Keycloak token while Keycloak is disabled;
/// - [`JWTCheckError::EmailNotVerified`] if a Keycloak token carries no verified e-mail;
/// - [`JWTCheckError::IdentityProviderUnavailable`] if the realm keys cannot be fetched;
/// - [`JWTCheckError::DatabaseError`] if the lookup in the database fails;
/// - [`JWTCheckError::UnknownUser`] if no active account matches the token.
pub async fn authenticate_token(
    jwt: &str,
    db_interface: &SmartDatabase,
    keycloak: Option<&KeycloakTokenVerifier>,
) -> Result<u64, JWTCheckError> {
    if jwt.is_empty() {
        eprintln!("No JWT token provided.");
        return Err(JWTCheckError::NoTokenProvided);
    }

    let header = decode_header(jwt).map_err(|err| {
        eprintln!("JWT header decode error: {err:?}");
        JWTCheckError::InvalidToken
    })?;
    if header.alg.family() == AlgorithmFamily::Hmac {
        return resolve_legacy_user(jwt, db_interface).await;
    }

    let verifier = keycloak.ok_or_else(|| {
        eprintln!("Keycloak token received but Keycloak is not configured on this API.");
        JWTCheckError::InvalidToken
    })?;
    let identity = verifier.verify(jwt).await?;

    let query_view = GetUserIdByEmailQueryView::new(&identity.email);
    match db_interface.fetch_scalar::<i32, _>(&query_view).await {
        Ok(user_id) => Ok(id_from_sql(user_id)),
        Err(ApiLibError::Database(DbError::NotFound)) => {
            eprintln!(
                "No active account matches the Keycloak identity {} ({}).",
                identity.subject, identity.email
            );
            Err(JWTCheckError::UnknownUser)
        }
        Err(e) => {
            eprintln!("Database query error: {e}");
            Err(JWTCheckError::DatabaseError)
        }
    }
}
