use crate::database::query_views::IsUserActiveByIdQueryView;
use crate::jwt_manager::decode_jwt::decode_jwt;
use crate::jwt_manager::error::JWTCheckError;
use crate::jwt_manager::session_revocation::is_session_revoked;
use crate::jwt_manager::validate_jwt_config::{is_session_required, REQUIRE_SESSION_ENV};
use crate::smart_db::SmartDatabase;

/// Checks that a JWT signed with `JWT_SECRET` is present, valid, not expired and designates an
/// existing, non-archived account.
///
/// Ne concerne que les jetons historiques émis par `Core_API` : pour accepter aussi les jetons
/// Keycloak, utiliser [`super::authenticate_token`].
///
/// # Errors
///
/// - [`JWTCheckError::NoTokenProvided`] si le jeton est vide ;
/// - [`JWTCheckError::ExpiredToken`] si le jeton est expiré ;
/// - [`JWTCheckError::InvalidToken`] if the token is unreadable, its `user_id` is not an integer,
///   or it has no `sid` while `JWT_REQUIRE_SESSION` is enabled;
/// - [`JWTCheckError::RevokedToken`] if the token's session (`sid` claim) was revoked;
/// - [`JWTCheckError::RevocationCheckUnavailable`] if the token has a `sid` and Redis cannot be
///   queried (fail closed);
/// - [`JWTCheckError::DatabaseError`] si la vérification en base échoue ;
/// - [`JWTCheckError::UnknownUser`] if the account does not exist or is archived.
pub async fn check_jwt_validity(
    jwt: &str,
    db_interface: &SmartDatabase,
) -> Result<(), JWTCheckError> {
    if jwt.is_empty() {
        tracing::debug!("No JWT token provided.");
        return Err(JWTCheckError::NoTokenProvided);
    }
    resolve_legacy_user(jwt, db_interface).await.map(|_| ())
}

/// Same checks as [`check_jwt_validity`] on a non-empty token, returning the `users.id` the
/// token designates.
pub(super) async fn resolve_legacy_user(
    jwt: &str,
    db_interface: &SmartDatabase,
) -> Result<u64, JWTCheckError> {
    // 1. Décodage et distinction de l'expiration vs token invalide
    let claims = decode_jwt(jwt).map_err(|err| {
        tracing::warn!(error = ?err, "JWT decode error");
        if matches!(
            err.kind(),
            jsonwebtoken::errors::ErrorKind::ExpiredSignature
        ) {
            JWTCheckError::ExpiredToken
        } else {
            JWTCheckError::InvalidToken
        }
    })?;

    // 2. Extraction et parsing de l'ID utilisateur
    let user_id_str = claims.user_id();
    let parsed_user_id: u64 = user_id_str.parse().map_err(|_| {
        tracing::warn!("Failed to parse user ID from JWT claims.");
        JWTCheckError::InvalidToken
    })?;

    // 2b. Revocation list: only tokens bound to a session can be revoked. When Redis cannot be
    // checked the token is refused (fail closed) with 503, not 401: the token itself is fine, the
    // client should retry rather than drop its session. Tokens without a session are refused
    // outright when `JWT_REQUIRE_SESSION` is enabled.
    if claims.session_id().is_none() && is_session_required() {
        tracing::info!("JWT without session (`sid`) refused: {REQUIRE_SESSION_ENV} is enabled.");
        return Err(JWTCheckError::InvalidToken);
    }
    if let Some(session_id) = claims.session_id() {
        let revoked = is_session_revoked(&db_interface.get_redis(), session_id)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Revocation lookup error");
                JWTCheckError::RevocationCheckUnavailable
            })?;
        if revoked {
            return Err(JWTCheckError::RevokedToken);
        }
    }

    // 3. The account must exist and not be archived.
    let query_view = IsUserActiveByIdQueryView::new(parsed_user_id);
    let exist = db_interface
        .fetch_scalar::<bool, _>(&query_view)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Database query error");
            JWTCheckError::DatabaseError
        })?;

    if !exist {
        tracing::info!(user_id = %user_id_str, "No active account with this ID");
        return Err(JWTCheckError::UnknownUser);
    }

    Ok(parsed_user_id)
}
