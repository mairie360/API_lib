use super::get_jwt_secret::get_jwt_secret;
use super::get_jwt_timeout::get_jwt_timeout;
use super::jwt_claims::Claims;
use super::session_keys::{session_signing_key, SESSION_JWT_AUDIENCE, SESSION_JWT_ISSUER};
use jsonwebtoken::errors::{Error, ErrorKind};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use std::time::{SystemTime, UNIX_EPOCH};

/// Signs a JWT for `user_id_str` with the role `role`, valid for `JWT_TIMEOUT` seconds.
///
/// The token carries no session id (`sid`): it cannot be revoked before it expires. Use
/// [`generate_session_jwt`] for tokens bound to a session.
///
/// # Errors
///
/// See [`generate_session_jwt`].
pub fn generate_jwt(user_id_str: &str, role: &str) -> Result<String, Error> {
    generate_session_jwt(user_id_str, role, None)
}

/// Same as [`generate_jwt`], with an optional session id stored in the `sid` claim.
///
/// A token with a `sid` is refused by every middleware of this crate once `revoked:<sid>`
/// exists in Redis (see [`revoke_session`](super::revoke_session)).
///
/// When `JWT_PRIVATE_KEY` is set (only on `Core_API`, MAIR-680), the token is signed with that
/// Ed25519 key (`alg` `EdDSA`, `kid` = thumbprint of its public key) and carries `iss` and `aud`.
/// Otherwise it is a historical HS256 token signed with `JWT_SECRET` (transition).
///
/// # Errors
///
/// Returns a `jsonwebtoken` error when `JWT_TIMEOUT` is missing or invalid, when
/// `JWT_PRIVATE_KEY` is invalid or not published in `JWT_PUBLIC_KEY`, when neither it nor
/// `JWT_SECRET` is set, or when encoding fails.
pub fn generate_session_jwt(
    user_id_str: &str,
    role: &str,
    session_id: Option<&str>,
) -> Result<String, Error> {
    let timeout = get_jwt_timeout()?;

    // A clock before the epoch gives 0: the token is then already expired (fails safe).
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    let expiration = usize::try_from(now)
        .unwrap_or(usize::MAX)
        .saturating_add(timeout);
    let mut claims = Claims::new(user_id_str, role, expiration);
    if let Some(session_id) = session_id {
        claims = claims.with_session_id(session_id);
    }

    if let Some(signing_key) =
        session_signing_key().map_err(|_| Error::from(ErrorKind::InvalidEddsaKey))?
    {
        let claims = claims.with_issuer_and_audience(SESSION_JWT_ISSUER, SESSION_JWT_AUDIENCE);
        let mut header = Header::new(Algorithm::EdDSA);
        header.kid = Some(signing_key.kid().to_string());
        return encode(&header, &claims, signing_key.encoding_key());
    }

    let secret: Vec<u8> = get_jwt_secret()?;
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(&secret),
    )
}
