use mairie360_api_lib::jwt_manager::{
    check_jwt_validity, generate_jwt, get_jwt_secret, get_jwt_timeout, get_user_id_from_jwt,
};
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;
use std::env;

/**
 * This module contains tests for the JWT manager.
 * It tests the functionality of generating JWTs, retrieving user IDs from JWTs,
 * and getting the JWT secret and timeout.
 */
static USER_ID: &str = "1";

/**
 * Global setup for tests.
 */
static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    // This code runs ONCE before any test
    env::set_var("JWT_SECRET", "b\"secret\"");
    env::set_var("JWT_TIMEOUT", "3600");
    println!("Global setup done");
});

/**
 * Sets up the environment for tests.
 * This function is called before each test to ensure the environment variables are set.
 */
fn setup() {
    // Force INIT to run
    std::sync::LazyLock::force(&INIT);
}

/**
 * Tests for the JWT manager.
 * These tests cover the generation of JWTs, retrieval of user IDs from JWTs,
 * and the retrieval of JWT secrets and timeouts.
 */
#[cfg(test)]
mod jwt_tests {
    use super::*;
    use mairie360_api_lib::{
        database::db_interface::Database,
        jwt_manager::{error::JWTCheckError, get_role_from_jwt},
        redis::redis_interface::Redis,
        smart_db::SmartDatabase,
    };

    /**
     * Tests the retrieval of the JWT secret.
     * It checks if the secret can be retrieved successfully and matches the expected value.
     * It also ensures that the secret is in the expected byte format.
     */
    #[test]
    fn test_get_jwt_secret() {
        setup();
        let secret = get_jwt_secret();
        assert!(
            secret.is_ok(),
            "Failed to get JWT secret: {:?}",
            secret.err()
        );
        assert_eq!(
            secret.unwrap(),
            b"b\"secret\"".to_vec(),
            "JWT secret does not match expected value"
        );
    }

    /**
     * Tests the retrieval of the JWT timeout.
     * It checks if the timeout can be retrieved successfully and matches the expected value.
     * The expected timeout is set to 3600 seconds (1 hour).
     */
    #[test]
    fn test_get_jwt_timeout() {
        setup();
        let timeout = get_jwt_timeout();
        assert!(
            timeout.is_ok(),
            "Failed to get JWT timeout: {:?}",
            timeout.err()
        );
        assert_eq!(
            timeout.unwrap(),
            3600,
            "JWT timeout does not match expected value"
        );
    }

    /**
     * Tests the generation of a JWT.
     * It checks if the JWT can be generated successfully for a given user ID.
     * The generated token should not be empty.
     * If the generation fails, it asserts with an error message.
     */
    #[test]
    fn test_generate_jwt() {
        setup();
        let token = generate_jwt(USER_ID, "test_role");
        assert!(token.is_ok(), "JWT generation failed: {:?}", token.err());
        let token = token.unwrap();
        assert!(!token.is_empty(), "Generated JWT token is empty");
    }

    /**
     * Tests the retrieval of a user ID from a JWT.
     * It checks if the user ID can be extracted from a valid JWT.
     * The user ID should match the expected value.
     * It also tests the case where an invalid JWT is provided, expecting None as the result
     */
    #[test]
    fn test_get_user_id_from_jwt() {
        setup();
        let token = generate_jwt(USER_ID, "test_role").unwrap();
        let user_id = get_user_id_from_jwt(&token);
        assert_eq!(
            user_id.unwrap(),
            USER_ID,
            "User ID does not match expected value"
        );
    }

    /**
     * Tests the retrieval of a user ID from an invalid JWT.
     * It checks if the function returns None when an invalid JWT is provided.
     * This ensures that the function handles invalid tokens gracefully.
     * If the function returns Some, it asserts with an error message.
     */
    #[test]
    fn test_get_user_id_from_invalid_jwt() {
        setup();
        let invalid_token = "invalid.token.string";
        let user_id = get_user_id_from_jwt(invalid_token);
        assert_eq!(user_id, None, "Expected None for invalid JWT, got Some");
    }

    /**
     * Tests the retrieval of a role from a JWT.
     * It checks if the role can be extracted from a valid JWT.
     * The role should match the expected value.
     */
    #[test]
    fn test_get_role_from_jwt() {
        setup();
        let expected_role = "admin";
        let token = generate_jwt(USER_ID, expected_role).unwrap();
        let role = get_role_from_jwt(&token);
        assert_eq!(
            role.as_deref(),
            Some(expected_role),
            "Role does not match expected value"
        );
    }

    /**
     * Tests the retrieval of a role from an invalid JWT.
     * It checks if the function returns None when an invalid JWT is provided.
     */
    #[test]
    fn test_get_role_from_invalid_jwt() {
        setup();
        let invalid_token = "invalid.token.string";
        let role = get_role_from_jwt(invalid_token);
        assert_eq!(role, None, "Expected None for invalid JWT, got Some");
    }

    #[tokio::test]
    #[serial]
    async fn test_jwt_check_valid() {
        setup();
        let (_container, host) = get_shared_db().await;
        let db_interface: Database = Database::new(host.as_str()).await;
        let redis: Redis = Redis::new("");
        let interface: SmartDatabase = SmartDatabase::new(db_interface, redis);
        let token = generate_jwt(USER_ID, "test_role").unwrap();
        assert!(
            check_jwt_validity(&token, &interface).await.is_ok(),
            "JWT validity check failed"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_jwt_check_empty_token() {
        setup();
        let (_container, host) = get_shared_db().await;
        let db_interface: Database = Database::new(host.as_str()).await;
        let redis: Redis = Redis::new("");
        let interface: SmartDatabase = SmartDatabase::new(db_interface, redis);
        assert_eq!(
            check_jwt_validity("", &interface).await.unwrap_err(),
            JWTCheckError::NoTokenProvided,
            "Expected error for invalid JWT"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_jwt_check_token_without_id() {
        setup();
        let (_container, host) = get_shared_db().await;
        let db_interface: Database = Database::new(host.as_str()).await;
        let redis: Redis = Redis::new("");
        let interface: SmartDatabase = SmartDatabase::new(db_interface, redis);
        let invalid_token = generate_jwt("", "test_role").unwrap();
        let result = check_jwt_validity(&invalid_token, &interface).await;
        assert!(result.is_err(), "Expected error for invalid JWT");
        let error = result.unwrap_err();
        assert_eq!(
            error,
            JWTCheckError::InvalidToken,
            "Expected error for invalid JWT"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_jwt_check_invalid_user_id() {
        setup();
        let (_container, host) = get_shared_db().await;
        let db_interface: Database = Database::new(host.as_str()).await;
        let redis: Redis = Redis::new("");
        let interface: SmartDatabase = SmartDatabase::new(db_interface, redis);
        let invalid_token = generate_jwt("8", "test_role").unwrap();
        assert_eq!(
            check_jwt_validity(&invalid_token, &interface)
                .await
                .unwrap_err(),
            JWTCheckError::UnknownUser,
            "Expected error for invalid JWT"
        );
    }
}

/**
 * Tests for `authenticate_token`, which accepts both the historical `JWT_SECRET` tokens and
 * the Keycloak access tokens (verified against a fake realm, see `test_setup::keycloak_setup`).
 */
#[cfg(test)]
mod authenticate_token_tests {
    use super::*;
    use mairie360_api_lib::{
        database::db_interface::Database,
        jwt_manager::{authenticate_token, error::JWTCheckError},
        keycloak::{KeycloakConfig, KeycloakTokenVerifier},
        redis::redis_interface::Redis,
        smart_db::SmartDatabase,
        test_setup::keycloak_setup::{access_token_claims, sign, KeycloakMock, TestKey, CLIENT_ID},
        test_setup::queries_setup::ALICE_ID,
    };
    use serde_json::json;

    async fn smart_db() -> SmartDatabase {
        let (_container, host) = get_shared_db().await;
        let db_interface: Database = Database::new(host.as_str()).await;
        SmartDatabase::new(db_interface, Redis::new(""))
    }

    fn alice_id() -> u64 {
        u64::try_from(*ALICE_ID.get().expect("Alice ID not initialized")).unwrap()
    }

    #[tokio::test]
    #[serial]
    async fn test_legacy_token_resolves_the_user_id() {
        setup();
        let db = smart_db().await;
        let token = generate_jwt(&alice_id().to_string(), "test_role").unwrap();

        let result = authenticate_token(&token, &db, None).await;

        assert_eq!(result, Ok(alice_id()));
    }

    #[tokio::test]
    #[serial]
    async fn test_legacy_token_of_an_unknown_user_is_refused() {
        setup();
        let db = smart_db().await;
        let token = generate_jwt("8", "test_role").unwrap();

        let result = authenticate_token(&token, &db, None).await;

        assert_eq!(result, Err(JWTCheckError::UnknownUser));
    }

    #[tokio::test]
    #[serial]
    async fn test_empty_and_malformed_tokens_are_refused() {
        setup();
        let db = smart_db().await;
        let mock = KeycloakMock::start();
        let verifier = mock.verifier();

        assert_eq!(
            authenticate_token("", &db, Some(&verifier)).await,
            Err(JWTCheckError::NoTokenProvided)
        );
        assert_eq!(
            authenticate_token("not.a.token", &db, Some(&verifier)).await,
            Err(JWTCheckError::InvalidToken)
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_keycloak_token_resolves_the_account_by_email() {
        setup();
        let db = smart_db().await;
        let mock = KeycloakMock::start();
        let verifier = mock.verifier();
        let token = mock.access_token("alice@example.com");

        let result = authenticate_token(&token, &db, Some(&verifier)).await;

        assert_eq!(result, Ok(alice_id()));
    }

    #[tokio::test]
    #[serial]
    async fn test_keycloak_email_match_ignores_case() {
        setup();
        let db = smart_db().await;
        let mock = KeycloakMock::start();
        let verifier = mock.verifier();
        let token = mock.access_token("Alice@Example.COM");

        let result = authenticate_token(&token, &db, Some(&verifier)).await;

        assert_eq!(result, Ok(alice_id()));
    }

    #[tokio::test]
    #[serial]
    async fn test_keycloak_token_is_refused_when_keycloak_is_disabled() {
        setup();
        let db = smart_db().await;
        let mock = KeycloakMock::start();
        let token = mock.access_token("alice@example.com");

        let result = authenticate_token(&token, &db, None).await;

        assert_eq!(result, Err(JWTCheckError::InvalidToken));
    }

    #[tokio::test]
    #[serial]
    async fn test_check_jwt_validity_only_accepts_legacy_tokens() {
        setup();
        let db = smart_db().await;
        let mock = KeycloakMock::start();
        let token = mock.access_token("alice@example.com");

        let result = check_jwt_validity(&token, &db).await;

        assert_eq!(result, Err(JWTCheckError::InvalidToken));
    }

    #[tokio::test]
    #[serial]
    async fn test_keycloak_token_without_matching_account_is_unknown() {
        setup();
        let db = smart_db().await;
        let mock = KeycloakMock::start();
        let verifier = mock.verifier();
        let token = mock.access_token("nobody@example.com");

        let result = authenticate_token(&token, &db, Some(&verifier)).await;

        assert_eq!(result, Err(JWTCheckError::UnknownUser));
    }

    #[tokio::test]
    #[serial]
    async fn test_keycloak_token_of_an_archived_account_is_unknown() {
        setup();
        let db = smart_db().await;
        let mock = KeycloakMock::start();
        let verifier = mock.verifier();
        // Bob is archived by the shared fixtures.
        let token = mock.access_token("bob@example.com");

        let result = authenticate_token(&token, &db, Some(&verifier)).await;

        assert_eq!(result, Err(JWTCheckError::UnknownUser));
    }

    #[tokio::test]
    #[serial]
    async fn test_expired_keycloak_token_is_expired() {
        setup();
        let db = smart_db().await;
        let mock = KeycloakMock::start();
        let verifier = mock.verifier();
        let mut claims = access_token_claims(mock.realm_url(), "alice@example.com");
        claims["exp"] = json!(jsonwebtoken::get_current_timestamp() - 300);
        let token = sign(&claims, TestKey::A, TestKey::A.kid());

        let result = authenticate_token(&token, &db, Some(&verifier)).await;

        assert_eq!(result, Err(JWTCheckError::ExpiredToken));
    }

    #[tokio::test]
    #[serial]
    async fn test_keycloak_token_without_verified_email_is_forbidden() {
        setup();
        let db = smart_db().await;
        let mock = KeycloakMock::start();
        let verifier = mock.verifier();
        let mut claims = access_token_claims(mock.realm_url(), "alice@example.com");
        claims["email_verified"] = json!(false);
        let token = sign(&claims, TestKey::A, TestKey::A.kid());

        let result = authenticate_token(&token, &db, Some(&verifier)).await;

        assert_eq!(result, Err(JWTCheckError::EmailNotVerified));
    }

    #[tokio::test]
    #[serial]
    async fn test_unreachable_realm_is_identity_provider_unavailable() {
        setup();
        let db = smart_db().await;
        let realm = "http://127.0.0.1:9/realms/down";
        let verifier =
            KeycloakTokenVerifier::new(KeycloakConfig::new(realm, None, CLIENT_ID, None));
        let claims = access_token_claims(realm, "alice@example.com");
        let token = sign(&claims, TestKey::A, TestKey::A.kid());

        let result = authenticate_token(&token, &db, Some(&verifier)).await;

        assert_eq!(result, Err(JWTCheckError::IdentityProviderUnavailable));
    }
}
