use crate::database::query_views::DoesUserExistByIdQueryView;
use crate::jwt_manager::decode_jwt::decode_jwt;
use crate::jwt_manager::error::JWTCheckError;
use crate::jwt_manager::session_revocation::is_session_revoked;
use crate::smart_db::SmartDatabase;

/// Vérifie qu'un JWT est présent, valide, non expiré et qu'il désigne un utilisateur existant.
///
/// # Errors
///
/// - [`JWTCheckError::NoTokenProvided`] si le jeton est vide ;
/// - [`JWTCheckError::ExpiredToken`] si le jeton est expiré ;
/// - [`JWTCheckError::InvalidToken`] si le jeton est illisible ou si son `user_id` n'est pas un entier ;
/// - [`JWTCheckError::RevokedToken`] if the token's session (`sid` claim) was revoked;
/// - [`JWTCheckError::RevocationCheckUnavailable`] if the token has a `sid` and Redis cannot be
///   queried (fail closed);
/// - [`JWTCheckError::DatabaseError`] si la vérification en base échoue ;
/// - [`JWTCheckError::UnknownUser`] si l'utilisateur n'existe pas.
pub async fn check_jwt_validity(
    jwt: &str,
    db_interface: &SmartDatabase,
) -> Result<(), JWTCheckError> {
    if jwt.is_empty() {
        eprintln!("No JWT token provided.");
        return Err(JWTCheckError::NoTokenProvided);
    }

    // 1. Décodage et distinction de l'expiration vs token invalide
    let claims = decode_jwt(jwt).map_err(|err| {
        eprintln!("JWT decode error: {err:?}");
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
        eprintln!("Failed to parse user ID from JWT claims.");
        JWTCheckError::InvalidToken
    })?;

    // 2b. Revocation list: only tokens bound to a session can be revoked. When Redis cannot be
    // checked the token is refused (fail closed) with 503, not 401: the token itself is fine, the
    // client should retry rather than drop its session.
    if let Some(session_id) = claims.session_id() {
        let revoked = is_session_revoked(&db_interface.get_redis(), session_id)
            .await
            .map_err(|e| {
                eprintln!("Revocation lookup error: {e}");
                JWTCheckError::RevocationCheckUnavailable
            })?;
        if revoked {
            return Err(JWTCheckError::RevokedToken);
        }
    }

    // 3. Vérification en base de données
    let query_view = DoesUserExistByIdQueryView::new(parsed_user_id);
    let exist = db_interface
        .fetch_scalar::<bool, _>(&query_view)
        .await
        .map_err(|e| {
            eprintln!("Database query error: {e}");
            JWTCheckError::DatabaseError
        })?;

    if !exist {
        eprintln!("User does not exist with ID: {user_id_str}");
        return Err(JWTCheckError::UnknownUser);
    }

    Ok(())
}
