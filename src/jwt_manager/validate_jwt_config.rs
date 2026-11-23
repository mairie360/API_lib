use super::session_keys::{session_public_keys, session_signing_key, SessionKeyError};
use crate::env_manager::get_env_var;
use thiserror::Error;

/// `x` (base64url) of the Ed25519 public keys committed in the repositories: the test key pair of
/// `test_setup` (`http_setup::TEST_JWT_PUBLIC_KEY`). Anybody can sign tokens
/// for a deployment that trusts them.
const KNOWN_TEST_PUBLIC_KEYS: [&str; 1] = ["KW8bV4kiFUo8hmOEU3-lXdqZL1wmKvBw6smUyjpmXks"];

/// Set to `true` (or `1`) to start with the committed test key pair in `JWT_PUBLIC_KEY`.
///
/// For the dev, ZAP, k6 and integration stacks only, never a real deployment. The name dates from
/// the shared HS256 secret; it is kept so that the stacks and the images need no lockstep rename.
pub const ALLOW_WEAK_SECRET_ENV: &str = "JWT_ALLOW_WEAK_SECRET";

/// Set to `true` (or `1`) to refuse tokens without a `sid` claim, which cannot be revoked before
/// they expire (see [`super::generate_session_jwt`]).
pub const REQUIRE_SESSION_ENV: &str = "JWT_REQUIRE_SESSION";

/// A defect of the `JWT_PUBLIC_KEY` / `JWT_PRIVATE_KEY` / `JWT_TIMEOUT` configuration.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum JwtConfigError {
    #[error("JWT_PUBLIC_KEY is not set")]
    MissingPublicKey,
    #[error(transparent)]
    InvalidSessionKey(#[from] SessionKeyError),
    #[error("JWT_PUBLIC_KEY holds a well-known test key")]
    KnownTestKey,
    #[error("JWT_TIMEOUT is not set")]
    MissingTimeout,
    #[error("JWT_TIMEOUT must be a positive number of seconds")]
    InvalidTimeout,
}

impl JwtConfigError {
    /// `JWT_PUBLIC_KEY` holds the committed test key: tolerated when [`ALLOW_WEAK_SECRET_ENV`]
    /// is enabled.
    #[must_use]
    pub const fn is_test_key(&self) -> bool {
        matches!(self, Self::KnownTestKey)
    }
}

fn is_enabled(name: &str) -> bool {
    get_env_var(name).is_some_and(|value| {
        let value = value.trim();
        value == "1" || value.eq_ignore_ascii_case("true")
    })
}

/// Whether tokens without `sid` must be refused ([`REQUIRE_SESSION_ENV`]).
#[must_use]
pub fn is_session_required() -> bool {
    is_enabled(REQUIRE_SESSION_ENV)
}

/// Checks the session token keys and `JWT_TIMEOUT` as read from the environment.
///
/// - `JWT_PUBLIC_KEY` is required and must hold Ed25519 public keys, none of them a committed
///   test key: it is the only way to verify a session token. `JWT_SECRET` is ignored, HS256
///   tokens are always refused;
/// - `JWT_PRIVATE_KEY`, when set (Core only), must be an Ed25519 private key whose public key is
///   in `JWT_PUBLIC_KEY`;
/// - the timeout must be a positive integer.
///
/// # Errors
///
/// Returns the first [`JwtConfigError`] found: keys, then timeout.
pub fn validate_jwt_config() -> Result<(), JwtConfigError> {
    let public_keys = session_public_keys()?;
    session_signing_key()?;
    if public_keys.is_empty() {
        return Err(JwtConfigError::MissingPublicKey);
    }
    if public_keys
        .iter()
        .any(|key| KNOWN_TEST_PUBLIC_KEYS.contains(&key.x()))
    {
        return Err(JwtConfigError::KnownTestKey);
    }

    let timeout = get_env_var("JWT_TIMEOUT").ok_or(JwtConfigError::MissingTimeout)?;
    match timeout.trim().parse::<usize>() {
        Ok(seconds) if seconds > 0 => Ok(()),
        _ => Err(JwtConfigError::InvalidTimeout),
    }
}

/// Runs [`validate_jwt_config`] at startup (called by [`crate::state::AppState`]).
///
/// The committed test key is only logged when [`ALLOW_WEAK_SECRET_ENV`] is enabled. Built with
/// the `test-utils` feature (test builds), every defect is only logged, so that test apps can
/// build an `AppState` without production keys.
///
/// # Panics
///
/// Panics on any other configuration defect, so that a misconfigured API never starts.
pub fn enforce_jwt_config() {
    let Err(error) = validate_jwt_config() else {
        return;
    };
    if error.is_test_key() && is_enabled(ALLOW_WEAK_SECRET_ENV) {
        tracing::warn!("{error}; accepted because {ALLOW_WEAK_SECRET_ENV} is set.");
        return;
    }
    if cfg!(feature = "test-utils") {
        tracing::warn!("{error}; accepted because the `test-utils` feature is on.");
        return;
    }
    panic!(
        "Invalid JWT configuration: {error}. Set JWT_PUBLIC_KEY to Core's Ed25519 public key(s), \
         JWT_PRIVATE_KEY on Core only ({ALLOW_WEAK_SECRET_ENV}=true tolerates the committed test \
         key pair on a dev/test stack only), and a positive JWT_TIMEOUT."
    );
}
