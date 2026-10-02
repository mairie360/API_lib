//! Startup validation of `JWT_SECRET` / `JWT_TIMEOUT` (MAIR-391). Own binary: the tests change
//! process-wide variables through `temp_env`, which serialises them.
use mairie360_api_lib::jwt_manager::{
    enforce_jwt_config, is_session_required, validate_jwt_config, JwtConfigError,
};

const STRONG_SECRET: &str = "0123456789abcdef0123456789abcdef";

fn validate_with(secret: Option<&str>, timeout: Option<&str>) -> Result<(), JwtConfigError> {
    temp_env::with_vars(
        [("JWT_SECRET", secret), ("JWT_TIMEOUT", timeout)],
        validate_jwt_config,
    )
}

#[test]
fn test_strong_configuration_is_valid() {
    assert_eq!(validate_with(Some(STRONG_SECRET), Some("3600")), Ok(()));
}

#[test]
fn test_missing_or_blank_secret_is_refused() {
    assert_eq!(
        validate_with(None, Some("3600")),
        Err(JwtConfigError::MissingSecret)
    );
    assert_eq!(
        validate_with(Some("   "), Some("3600")),
        Err(JwtConfigError::MissingSecret)
    );
}

#[test]
fn test_short_secret_is_refused() {
    assert_eq!(
        validate_with(Some("0123456789abcdef0123456789abcde"), Some("3600")),
        Err(JwtConfigError::SecretTooShort(31))
    );
}

#[test]
fn test_known_test_secrets_are_refused() {
    for secret in ["b\"secret\"", "secret", "Test-Only", "e2e-jwt-not-a-secret"] {
        assert_eq!(
            validate_with(Some(secret), Some("3600")),
            Err(JwtConfigError::KnownTestSecret),
            "{secret}"
        );
    }
}

#[test]
fn test_missing_or_invalid_timeout_is_refused() {
    assert_eq!(
        validate_with(Some(STRONG_SECRET), None),
        Err(JwtConfigError::MissingTimeout)
    );
    for timeout in ["0", "-1", "one hour", ""] {
        assert_eq!(
            validate_with(Some(STRONG_SECRET), Some(timeout)),
            Err(JwtConfigError::InvalidTimeout),
            "{timeout}"
        );
    }
}

#[test]
fn test_only_secret_defects_are_weak() {
    assert!(JwtConfigError::SecretTooShort(3).is_weak_secret());
    assert!(JwtConfigError::KnownTestSecret.is_weak_secret());
    assert!(!JwtConfigError::MissingSecret.is_weak_secret());
    assert!(!JwtConfigError::MissingTimeout.is_weak_secret());
    assert!(!JwtConfigError::InvalidTimeout.is_weak_secret());
}

#[test]
fn test_enforcement_only_logs_in_test_builds() {
    // The `test-utils` feature (enabled for this crate's tests) downgrades every defect to a log,
    // so that test apps can build an `AppState` without a production secret.
    temp_env::with_vars(
        [
            ("JWT_SECRET", None::<&str>),
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
