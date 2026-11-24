//! Startup validation of `JWT_PUBLIC_KEY` / `JWT_TIMEOUT` (MAIR-391, MAIR-680). Own binary: the
//! tests change process-wide variables through `temp_env`, which serialises them.
use mairie360_api_lib::jwt_manager::{
    enforce_jwt_config, is_session_required, validate_jwt_config, JwtConfigError,
};
use mairie360_api_lib::test_setup::http_setup::TEST_JWT_PUBLIC_KEY;

/// An Ed25519 public key that is not the committed test key (its private half was discarded).
const PUBLIC_KEY: &str = "-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEA9inv/klLOJXSqVX0pkmQWtRR8riFopeTA2cpIsavYv4=
-----END PUBLIC KEY-----
";

fn validate_with(public_key: Option<&str>, timeout: Option<&str>) -> Result<(), JwtConfigError> {
    temp_env::with_vars(
        [
            ("JWT_PUBLIC_KEY", public_key),
            ("JWT_PRIVATE_KEY", None),
            ("JWT_SECRET", None),
            ("JWT_TIMEOUT", timeout),
        ],
        validate_jwt_config,
    )
}

#[test]
fn test_public_key_configuration_is_valid() {
    assert_eq!(validate_with(Some(PUBLIC_KEY), Some("3600")), Ok(()));
}

#[test]
fn test_missing_or_blank_public_key_is_refused() {
    assert_eq!(
        validate_with(None, Some("3600")),
        Err(JwtConfigError::MissingPublicKey)
    );
    assert_eq!(
        validate_with(Some("   "), Some("3600")),
        Err(JwtConfigError::MissingPublicKey)
    );
}

#[test]
fn test_mair_680_jwt_secret_no_longer_replaces_the_public_key() {
    // A strong JWT_SECRET used to be a complete configuration (HS256 transition): it is ignored now.
    let result = temp_env::with_vars(
        [
            ("JWT_PUBLIC_KEY", None),
            ("JWT_PRIVATE_KEY", None),
            ("JWT_SECRET", Some("0123456789abcdef0123456789abcdef")),
            ("JWT_TIMEOUT", Some("3600")),
        ],
        validate_jwt_config,
    );
    assert_eq!(result, Err(JwtConfigError::MissingPublicKey));
}

#[test]
fn test_committed_test_key_is_refused() {
    assert_eq!(
        validate_with(Some(TEST_JWT_PUBLIC_KEY), Some("3600")),
        Err(JwtConfigError::KnownTestKey)
    );
}

#[test]
fn test_missing_or_invalid_timeout_is_refused() {
    assert_eq!(
        validate_with(Some(PUBLIC_KEY), None),
        Err(JwtConfigError::MissingTimeout)
    );
    for timeout in ["0", "-1", "one hour", ""] {
        assert_eq!(
            validate_with(Some(PUBLIC_KEY), Some(timeout)),
            Err(JwtConfigError::InvalidTimeout),
            "{timeout}"
        );
    }
}

#[test]
fn test_only_the_test_key_is_tolerable() {
    assert!(JwtConfigError::KnownTestKey.is_test_key());
    assert!(!JwtConfigError::MissingPublicKey.is_test_key());
    assert!(!JwtConfigError::MissingTimeout.is_test_key());
    assert!(!JwtConfigError::InvalidTimeout.is_test_key());
}

#[test]
fn test_enforcement_only_logs_in_test_builds() {
    // The `test-utils` feature (enabled for this crate's tests) downgrades every defect to a log,
    // so that test apps can build an `AppState` without production keys.
    temp_env::with_vars(
        [
            ("JWT_PUBLIC_KEY", None::<&str>),
            ("JWT_PRIVATE_KEY", None),
            ("JWT_TIMEOUT", None),
            ("JWT_ALLOW_WEAK_SECRET", None),
        ],
        enforce_jwt_config,
    );
}

#[test]
fn test_session_requirement_is_opt_in() {
    temp_env::with_var("JWT_REQUIRE_SESSION", None::<&str>, || {
        assert!(!is_session_required());
    });
    for value in ["true", "TRUE", "1", " true "] {
        temp_env::with_var("JWT_REQUIRE_SESSION", Some(value), || {
            assert!(is_session_required(), "{value}");
        });
    }
    for value in ["false", "0", "yes", ""] {
        temp_env::with_var("JWT_REQUIRE_SESSION", Some(value), || {
            assert!(!is_session_required(), "{value}");
        });
    }
}
