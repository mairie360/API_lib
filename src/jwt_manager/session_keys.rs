//! Asymmetric keys of the session JWTs (MAIR-680).
//!
//! Only `Core_API` signs session tokens, with the Ed25519 private key of [`PRIVATE_KEY_ENV`].
//! Every other service only holds the public keys of [`PUBLIC_KEY_ENV`]: it can check a token,
//! never forge one. Both variables hold PEM text (`openssl genpkey -algorithm ed25519` and
//! `openssl pkey -pubout`); [`PUBLIC_KEY_ENV`] may hold several `PUBLIC KEY` blocks so that a key
//! can be rotated (the new key is published everywhere first, then Core signs with it).
//!
//! Each public key is identified by its RFC 7638 JWK thumbprint, sent as the `kid` header of the
//! tokens Core signs: the verifier picks the key by `kid`, and a token whose `kid` is unknown is
//! not a Core token (see [`super::authenticate_token`]).
//!
//! Tokens signed with these keys always carry `iss` = [`SESSION_JWT_ISSUER`] and `aud` =
//! [`SESSION_JWT_AUDIENCE`], both checked by [`super::decode_jwt`].

use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::env_manager::get_env_var;

/// PEM public key(s) every service verifies session tokens with.
pub const PUBLIC_KEY_ENV: &str = "JWT_PUBLIC_KEY";

/// PEM private key Core signs session tokens with. Only `Core_API` may have it.
pub const PRIVATE_KEY_ENV: &str = "JWT_PRIVATE_KEY";

/// `iss` claim of the session tokens signed by Core.
pub const SESSION_JWT_ISSUER: &str = "mairie360-core";

/// `aud` claim of the session tokens signed by Core: every API and BFF of the platform.
pub const SESSION_JWT_AUDIENCE: &str = "mairie360";

/// DER prefix of an Ed25519 `SubjectPublicKeyInfo`, followed by the 32-byte key.
const ED25519_SPKI_PREFIX: [u8; 12] = [
    0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
];

/// DER prefix of an Ed25519 PKCS#8 v1 private key, followed by the 32-byte seed.
const ED25519_PKCS8_PREFIX: [u8; 16] = [
    0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04, 0x20,
];

/// A defect of [`PUBLIC_KEY_ENV`] or [`PRIVATE_KEY_ENV`].
#[derive(Debug, Error, PartialEq, Eq)]
pub enum SessionKeyError {
    #[error("JWT_PUBLIC_KEY holds no Ed25519 `PUBLIC KEY` PEM block")]
    InvalidPublicKey,
    #[error("JWT_PRIVATE_KEY is not an Ed25519 `PRIVATE KEY` PEM block")]
    InvalidPrivateKey,
    #[error("JWT_PRIVATE_KEY is set but JWT_PUBLIC_KEY is not")]
    MissingPublicKey,
    #[error("JWT_PRIVATE_KEY does not match any key of JWT_PUBLIC_KEY")]
    UnpublishedPrivateKey,
}

/// A public key session tokens are verified with.
#[derive(Clone)]
pub struct SessionPublicKey {
    kid: String,
    x: String,
    key: DecodingKey,
}

impl SessionPublicKey {
    /// RFC 7638 thumbprint of the key, the `kid` of the tokens it verifies.
    #[must_use]
    pub fn kid(&self) -> &str {
        &self.kid
    }

    /// The raw 32-byte key, base64url without padding (the JWK `x`).
    #[must_use]
    pub fn x(&self) -> &str {
        &self.x
    }

    /// The key, for [`jsonwebtoken::decode`].
    #[must_use]
    pub const fn decoding_key(&self) -> &DecodingKey {
        &self.key
    }

    /// The key as a public JWK (`kty` OKP, `crv` Ed25519).
    #[must_use]
    pub fn jwk(&self) -> serde_json::Value {
        serde_json::json!({
            "kty": "OKP",
            "crv": "Ed25519",
            "x": self.x,
            "kid": self.kid,
            "alg": "EdDSA",
            "use": "sig",
        })
    }
}

/// The private key Core signs session tokens with, and the `kid` of its public key.
#[derive(Clone)]
pub struct SessionSigningKey {
    kid: String,
    key: EncodingKey,
}

impl SessionSigningKey {
    /// `kid` header of the tokens signed with this key.
    #[must_use]
    pub fn kid(&self) -> &str {
        &self.kid
    }

    /// The key, for [`jsonwebtoken::encode`].
    #[must_use]
    pub const fn encoding_key(&self) -> &EncodingKey {
        &self.key
    }
}

/// The base64 bodies of the `label` PEM blocks of `text`, decoded. Environment files often
/// escape the line breaks of a PEM value as `\n`: both forms are accepted.
fn pem_blocks(text: &str, label: &str) -> Vec<Vec<u8>> {
    let text = text.replace("\\n", "\n");
    let begin = format!("-----BEGIN {label}-----");
    let end = format!("-----END {label}-----");
    let mut blocks = Vec::new();
    let mut body: Option<String> = None;
    for line in text.lines().map(str::trim) {
        if line == begin {
            body = Some(String::new());
        } else if line == end {
            if let Some(content) = body.take() {
                if let Ok(der) = STANDARD.decode(content) {
                    blocks.push(der);
                }
            }
        } else if let Some(content) = body.as_mut() {
            content.push_str(line);
        }
    }
    blocks
}

/// RFC 7638 thumbprint of the Ed25519 public key whose base64url `x` is given.
fn thumbprint(x: &str) -> String {
    let canonical = format!(r#"{{"crv":"Ed25519","kty":"OKP","x":"{x}"}}"#);
    URL_SAFE_NO_PAD.encode(Sha256::digest(canonical.as_bytes()))
}

/// Parses the Ed25519 public keys of a PEM text (several `PUBLIC KEY` blocks allowed).
///
/// # Errors
///
/// [`SessionKeyError::InvalidPublicKey`] when no block is an Ed25519 public key, or one block
/// is not.
pub fn parse_public_keys(pem: &str) -> Result<Vec<SessionPublicKey>, SessionKeyError> {
    let blocks = pem_blocks(pem, "PUBLIC KEY");
    if blocks.is_empty() {
        return Err(SessionKeyError::InvalidPublicKey);
    }
    blocks
        .iter()
        .map(|der| {
            let raw = der
                .strip_prefix(&ED25519_SPKI_PREFIX[..])
                .filter(|raw| raw.len() == 32)
                .ok_or(SessionKeyError::InvalidPublicKey)?;
            let x = URL_SAFE_NO_PAD.encode(raw);
            Ok(SessionPublicKey {
                kid: thumbprint(&x),
                x,
                key: DecodingKey::from_ed_der(raw),
            })
        })
        .collect()
}

/// Parses an Ed25519 private key and finds, among `public_keys`, the one it matches.
///
/// # Errors
///
/// [`SessionKeyError::InvalidPrivateKey`] when `pem` is not one Ed25519 `PRIVATE KEY` block,
/// [`SessionKeyError::UnpublishedPrivateKey`] when no public key matches it: the tokens it would
/// sign could not be verified by anyone.
pub fn parse_signing_key(
    pem: &str,
    public_keys: &[SessionPublicKey],
) -> Result<SessionSigningKey, SessionKeyError> {
    let blocks = pem_blocks(pem, "PRIVATE KEY");
    let [der] = blocks.as_slice() else {
        return Err(SessionKeyError::InvalidPrivateKey);
    };
    if der.len() != ED25519_PKCS8_PREFIX.len() + 32 || !der.starts_with(&ED25519_PKCS8_PREFIX) {
        return Err(SessionKeyError::InvalidPrivateKey);
    }
    let key = EncodingKey::from_ed_der(der);
    // Sign a probe and keep the public key that verifies it: no need to derive the public key.
    let probe = b"mairie360 session key probe";
    let signature = jsonwebtoken::crypto::sign(probe, &key, Algorithm::EdDSA)
        .map_err(|_| SessionKeyError::InvalidPrivateKey)?;
    let public = public_keys
        .iter()
        .find(|public| {
            jsonwebtoken::crypto::verify(&signature, probe, public.decoding_key(), Algorithm::EdDSA)
                .unwrap_or(false)
        })
        .ok_or(SessionKeyError::UnpublishedPrivateKey)?;
    Ok(SessionSigningKey {
        kid: public.kid.clone(),
        key,
    })
}

/// The public keys of [`PUBLIC_KEY_ENV`]; empty when the variable is unset or blank.
///
/// # Errors
///
/// [`SessionKeyError::InvalidPublicKey`] when the variable is set but malformed.
pub fn session_public_keys() -> Result<Vec<SessionPublicKey>, SessionKeyError> {
    get_env_var(PUBLIC_KEY_ENV)
        .filter(|value| !value.trim().is_empty())
        .map_or_else(|| Ok(Vec::new()), |pem| parse_public_keys(&pem))
}

/// The signing key of [`PRIVATE_KEY_ENV`], or `None` when the variable is unset or blank (the
/// service does not sign, or Core still signs with `JWT_SECRET` during the transition).
///
/// # Errors
///
/// See [`parse_signing_key`]; [`SessionKeyError::MissingPublicKey`] when [`PUBLIC_KEY_ENV`] is
/// not set.
pub fn session_signing_key() -> Result<Option<SessionSigningKey>, SessionKeyError> {
    let Some(pem) = get_env_var(PRIVATE_KEY_ENV).filter(|value| !value.trim().is_empty()) else {
        return Ok(None);
    };
    let public_keys = session_public_keys()?;
    if public_keys.is_empty() {
        return Err(SessionKeyError::MissingPublicKey);
    }
    parse_signing_key(&pem, &public_keys).map(Some)
}

/// The JWKS (`{"keys": [...]}`) of [`PUBLIC_KEY_ENV`], for Core to publish.
///
/// # Errors
///
/// [`SessionKeyError::InvalidPublicKey`] when the variable is set but malformed.
pub fn session_jwks() -> Result<serde_json::Value, SessionKeyError> {
    let keys: Vec<_> = session_public_keys()?
        .iter()
        .map(SessionPublicKey::jwk)
        .collect();
    Ok(serde_json::json!({ "keys": keys }))
}
