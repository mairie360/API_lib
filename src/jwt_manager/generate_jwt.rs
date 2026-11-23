use super::get_jwt_timeout::get_jwt_timeout;
use super::jwt_claims::Claims;
use super::session_keys::{session_signing_key, SESSION_JWT_AUDIENCE, SESSION_JWT_ISSUER};
use jsonwebtoken::errors::{Error, ErrorKind};
use jsonwebtoken::{encode, Algorithm, Header};
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
/// The token is signed with the Ed25519 key of `JWT_PRIVATE_KEY` (`alg` `EdDSA`, `kid` =
/// thumbprint of its public key) and carries `iss` and `aud`. Only `Core_API` holds that key.
///
/// # Errors
///
/// Returns a `jsonwebtoken` error when `JWT_TIMEOUT` is missing or invalid, when
/// `JWT_PRIVATE_KEY` is not set (`MissingRequiredClaim`), invalid or not published in
/// `JWT_PUBLIC_KEY`, or when encoding fails.
pub fn generate_session_jwt(
    user_id_str: &str,
    role: &str,
    session_id: Option<&str>,
) -> Result<String, Error> {
    generate_bound_session_jwt(user_id_str, role, session_id, &SessionBinding::default())
}

/// What a session token carries about the device and the network it was opened from (MAIR-773).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionBinding {
    /// RFC 7638 thumbprint of the device key (`DPoP`), written as `cnf.jkt`.
    pub device_key_thumbprint: Option<String>,
    /// Network prefix of the sign-in address, written as `net`.
    pub network: Option<String>,
}

/// Same as [`generate_session_jwt`], with the device key and the sign-in network of the session.
///
/// # Errors
///
/// See [`generate_session_jwt`].
pub fn generate_bound_session_jwt(
    user_id_str: &str,
    role: &str,
    session_id: Option<&str>,
    binding: &SessionBinding,
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
    if let Some(jkt) = binding.device_key_thumbprint.as_deref() {
        claims = claims.with_device_key(jkt);
    }
    if let Some(network) = binding.network.as_deref() {
        claims = claims.with_network(network);
    }

    let signing_key = session_signing_key()
        .map_err(|_| Error::from(ErrorKind::InvalidEddsaKey))?
        .ok_or_else(|| {
            Error::from(ErrorKind::MissingRequiredClaim(
                "JWT_PRIVATE_KEY is not set: only Core_API signs session tokens".to_string(),
            ))
        })?;
    let claims = claims.with_issuer_and_audience(SESSION_JWT_ISSUER, SESSION_JWT_AUDIENCE);
    let mut header = Header::new(Algorithm::EdDSA);
    header.kid = Some(signing_key.kid().to_string());
    encode(&header, &claims, signing_key.encoding_key())
}
