use super::session_keys::{session_public_keys, session_signing_key, SessionKeyError};
use crate::env_manager::get_env_var;
use thiserror::Error;

/// Minimum length of `JWT_SECRET`, in bytes: 256 bits, the size of the HS256 key.
pub const MIN_JWT_SECRET_LEN: usize = 32;

/// Secrets found in the repositories (compose stacks, chart tests, e2e values) or commonly used
/// as placeholders. A deployment using one of them signs tokens anybody can forge.
const KNOWN_TEST_SECRETS: [&str; 8] = [
    "b\"secret\"",
    "secret",
    "test-only",
    "e2e-jwt-not-a-secret",
    "changeme",
    "change-me",
    "jwt_secret",
    "your-secret-key",
];

/// `x` (base64url) of the Ed25519 public keys committed in the repositories: the test key pair of
/// `test_setup` (`http_setup::TEST_JWT_PUBLIC_KEY`). Anybody can sign tokens
/// for a deployment that trusts them.
const KNOWN_TEST_PUBLIC_KEYS: [&str; 1] = ["KW8bV4kiFUo8hmOEU3-lXdqZL1wmKvBw6smUyjpmXks"];

/// Set to `true` (or `1`) to start with a weak `JWT_SECRET` (dev, ZAP, k6 and integration
/// stacks). Never set it in a real deployment.
pub const ALLOW_WEAK_SECRET_ENV: &str = "JWT_ALLOW_WEAK_SECRET";

/// Set to `true` (or `1`) to refuse historical tokens without a `sid` claim, which cannot be
/// revoked before they expire (see [`super::generate_session_jwt`]).
pub const REQUIRE_SESSION_ENV: &str = "JWT_REQUIRE_SESSION";

/// A defect of the `JWT_PUBLIC_KEY` / `JWT_PRIVATE_KEY` / `JWT_SECRET` / `JWT_TIMEOUT`
/// configuration.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum JwtConfigError {
    #[error("neither JWT_PUBLIC_KEY nor JWT_SECRET is set")]
    MissingSecret,
    #[error(transparent)]
    InvalidSessionKey(#[from] SessionKeyError),
    #[error("JWT_PUBLIC_KEY holds a well-known test key")]
    KnownTestKey,
    #[error("JWT_SECRET is {0} bytes long, at least {MIN_JWT_SECRET_LEN} are required")]
    SecretTooShort(usize),
    #[error("JWT_SECRET is a well-known test value")]
    KnownTestSecret,
    #[error("JWT_TIMEOUT is not set")]
    MissingTimeout,
    #[error("JWT_TIMEOUT must be a positive number of seconds")]
    InvalidTimeout,
}

impl JwtConfigError {
    /// The secret is set but weak: tolerated when [`ALLOW_WEAK_SECRET_ENV`] is enabled.
    #[must_use]
    pub const fn is_weak_secret(&self) -> bool {
        matches!(
            self,
            Self::SecretTooShort(_) | Self::KnownTestSecret | Self::KnownTestKey
        )
    }
}

fn is_enabled(name: &str) -> bool {
    get_env_var(name).is_some_and(|value| {
        let value = value.trim();
        value == "1" || value.eq_ignore_ascii_case("true")
    })
}

/// Whether historical tokens without `sid` must be refused ([`REQUIRE_SESSION_ENV`]).
#[must_use]
pub fn is_session_required() -> bool {
    is_enabled(REQUIRE_SESSION_ENV)
}

/// Checks the session token keys and `JWT_TIMEOUT` as read from the environment (MAIR-680).
///
/// - `JWT_PUBLIC_KEY`, when set, must hold Ed25519 public keys, none of them a committed test
///   key;
/// - `JWT_PRIVATE_KEY`, when set (Core only), must be an Ed25519 private key whose public key is
///   in `JWT_PUBLIC_KEY`;
/// - `JWT_SECRET` is optional once `JWT_PUBLIC_KEY` is set; when set (historical HS256 tokens,
///   transition), it must be at least [`MIN_JWT_SECRET_LEN`] bytes long and not a well-known
///   test value;
/// - the timeout must be a positive integer.
///
/// # Errors
///
/// Returns the first [`JwtConfigError`] found: keys, then secret, then timeout.
pub fn validate_jwt_config() -> Result<(), JwtConfigError> {
    let public_keys = session_public_keys()?;
    session_signing_key()?;
    if public_keys
        .iter()
        .any(|key| KNOWN_TEST_PUBLIC_KEYS.contains(&key.x()))
    {
        return Err(JwtConfigError::KnownTestKey);
    }

    match get_env_var("JWT_SECRET").filter(|secret| !secret.trim().is_empty()) {
        Some(secret) => {
            let normalized = secret.trim().to_ascii_lowercase();
            if KNOWN_TEST_SECRETS.contains(&normalized.as_str()) {
                return Err(JwtConfigError::KnownTestSecret);
            }
            if secret.len() < MIN_JWT_SECRET_LEN {
                return Err(JwtConfigError::SecretTooShort(secret.len()));
            }
        }
        None if public_keys.is_empty() => return Err(JwtConfigError::MissingSecret),
        None => {}
    }

    let timeout = get_env_var("JWT_TIMEOUT").ok_or(JwtConfigError::MissingTimeout)?;
    match timeout.trim().parse::<usize>() {
        Ok(seconds) if seconds > 0 => Ok(()),
        _ => Err(JwtConfigError::InvalidTimeout),
    }
}

/// Runs [`validate_jwt_config`] at startup (called by [`crate::state::AppState`]).
///
/// A weak secret is only logged when [`ALLOW_WEAK_SECRET_ENV`] is enabled. Built with the
/// `test-utils` feature (test builds), every defect is only logged, so that test apps can build
/// an `AppState` without a production secret.
///
/// # Panics
///
/// Panics on any other configuration defect, so that a misconfigured API never starts.
pub fn enforce_jwt_config() {
    let Err(error) = validate_jwt_config() else {
        return;
    };
    if error.is_weak_secret() && is_enabled(ALLOW_WEAK_SECRET_ENV) {
        tracing::warn!("{error}; accepted because {ALLOW_WEAK_SECRET_ENV} is set.");
        return;
    }
    if cfg!(feature = "test-utils") {
        tracing::warn!("{error}; accepted because the `test-utils` feature is on.");
        return;
    }
    panic!(
        "Invalid JWT configuration: {error}. Set JWT_PUBLIC_KEY to Core's Ed25519 public key(s) \
         (and JWT_PRIVATE_KEY on Core only), or during the transition a random JWT_SECRET of at \
         least {MIN_JWT_SECRET_LEN} bytes ({ALLOW_WEAK_SECRET_ENV}=true tolerates test keys and \
         secrets on a dev/test stack only), and a positive JWT_TIMEOUT."
    );
}
