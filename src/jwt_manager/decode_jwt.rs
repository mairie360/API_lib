use super::jwt_claims::Claims;
use super::session_keys::{session_public_keys, SESSION_JWT_AUDIENCE, SESSION_JWT_ISSUER};
use jsonwebtoken::errors::{Error, ErrorKind};
use jsonwebtoken::{decode, decode_header, Algorithm, Validation};

/// Decodes and validates a session JWT issued by `Core_API` (signature and expiry, no leeway).
///
/// Only `EdDSA` tokens signed with Core's Ed25519 key are accepted: the key is the one of
/// `JWT_PUBLIC_KEY` whose thumbprint is the `kid` header, `iss` must be
/// [`SESSION_JWT_ISSUER`](super::SESSION_JWT_ISSUER) and `aud`
/// [`SESSION_JWT_AUDIENCE`](super::SESSION_JWT_AUDIENCE). Any other algorithm is refused, the
/// HMAC ones included: no service holds a shared secret any more, so an HS256 token can only be
/// forged.
///
/// # Errors
///
/// Returns a `jsonwebtoken` error if the token is unreadable, uses another algorithm, its key is
/// not configured on this service (`InvalidKeyFormat`), its `kid` is unknown, the signature is
/// invalid, `iss`/`aud` do not match or the token is expired (`ExpiredSignature`).
pub fn decode_jwt(token: &str) -> Result<Claims, Error> {
    let header = decode_header(token)?;
    match header.alg {
        Algorithm::EdDSA => {
            let keys =
                session_public_keys().map_err(|_| Error::from(ErrorKind::InvalidKeyFormat))?;
            let key = header
                .kid
                .as_deref()
                .and_then(|kid| keys.iter().find(|key| key.kid() == kid))
                .ok_or_else(|| Error::from(ErrorKind::InvalidKeyFormat))?;
            let mut validation = Validation::new(Algorithm::EdDSA);
            validation.leeway = 0;
            validation.set_issuer(&[SESSION_JWT_ISSUER]);
            validation.set_audience(&[SESSION_JWT_AUDIENCE]);
            validation.set_required_spec_claims(&["exp", "sub", "iss", "aud"]);
            Ok(decode::<Claims>(token, key.decoding_key(), &validation)?.claims)
        }
        _ => Err(Error::from(ErrorKind::InvalidAlgorithm)),
    }
}
