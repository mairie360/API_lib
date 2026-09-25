use super::get_jwt_secret::get_jwt_secret;
use super::get_jwt_timeout::get_jwt_timeout;
use super::jwt_claims::Claims;
use jsonwebtoken::{encode, EncodingKey, Header};
use std::time::{SystemTime, UNIX_EPOCH};

/// Génère un JWT signé pour `user_id_str` avec le rôle `role`, valable `JWT_TIMEOUT` secondes.
///
/// The token carries no session id (`sid`): it cannot be revoked before it expires. Use
/// [`generate_session_jwt`] for tokens bound to a session.
///
/// # Errors
///
/// Renvoie une erreur `jsonwebtoken` si `JWT_SECRET` ou `JWT_TIMEOUT` est absent ou invalide,
/// ou si l'encodage échoue.
pub fn generate_jwt(user_id_str: &str, role: &str) -> Result<String, jsonwebtoken::errors::Error> {
    generate_session_jwt(user_id_str, role, None)
}

/// Same as [`generate_jwt`], with an optional session id stored in the `sid` claim.
///
/// A token with a `sid` is refused by every middleware of this crate once `revoked:<sid>`
/// exists in Redis (see [`revoke_session`](super::revoke_session)).
///
/// # Errors
///
/// Returns a `jsonwebtoken` error when `JWT_SECRET` or `JWT_TIMEOUT` is missing or invalid, or
/// when encoding fails.
pub fn generate_session_jwt(
    user_id_str: &str,
    role: &str,
    session_id: Option<&str>,
) -> Result<String, jsonwebtoken::errors::Error> {
    let secret: Vec<u8> = get_jwt_secret()?;
    let timeout = get_jwt_timeout()?;

    // Une horloge antérieure à l'epoch donne 0 : le jeton est alors déjà expiré (échec sûr).
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    // Token valid for the configured JWT timeout duration
    let expiration = usize::try_from(now)
        .unwrap_or(usize::MAX)
        .saturating_add(timeout);
    let mut claims = Claims::new(user_id_str, role, expiration);
    if let Some(session_id) = session_id {
        claims = claims.with_session_id(session_id);
    }
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(&secret),
    )?;
    Ok(token)
}
