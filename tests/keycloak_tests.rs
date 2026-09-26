use actix_web::{http::StatusCode, ResponseError};
use jsonwebtoken::{encode, get_current_timestamp, EncodingKey, Header};
use mairie360_api_lib::jwt_manager::error::JWTCheckError;
use mairie360_api_lib::keycloak::{KeycloakConfig, KeycloakError, KeycloakIdentity};
use mairie360_api_lib::test_setup::keycloak_setup::{
    access_token_claims, sign, KeycloakMock, TestKey, CLIENT_ID, SUBJECT,
};
use serde_json::{json, Value};

/**
 * This module contains tests for the keycloak module: the configuration read from the
 * environment and the verification of access tokens against a fake realm (JWKS served by a
 * local HTTP server, throwaway RSA keys). None of them needs Docker.
 */
const EMAIL: &str = "claire.martin@mairie360.test";

fn token_with(mock: &KeycloakMock, change: impl FnOnce(&mut Value)) -> String {
    let mut claims = access_token_claims(mock.realm_url(), EMAIL);
    change(&mut claims);
    sign(&claims, TestKey::A, TestKey::A.kid())
}

#[cfg(test)]
mod config_tests {
    use super::*;

    const REALM: &str = "https://auth.mairie360.fr/realms/mairie360";

    #[test]
    fn test_from_env_without_variables_is_disabled() {
        temp_env::with_vars(
            [
                ("KEYCLOAK_REALM_URL", None::<&str>),
                ("KEYCLOAK_CLIENT_ID", None),
                ("KEYCLOAK_CLIENT_SECRET", None),
                ("KEYCLOAK_ISSUER", None),
                ("KEYCLOAK_AUDIENCE", None),
            ],
            || assert_eq!(KeycloakConfig::from_env(), None),
        );
    }

    #[test]
    fn test_from_env_with_half_of_the_pair_is_disabled() {
        temp_env::with_vars(
            [
                ("KEYCLOAK_REALM_URL", Some(REALM)),
                ("KEYCLOAK_CLIENT_ID", None),
            ],
            || assert_eq!(KeycloakConfig::from_env(), None),
        );
        temp_env::with_vars(
            [
                ("KEYCLOAK_REALM_URL", Some("   ")),
                ("KEYCLOAK_CLIENT_ID", Some("core-api")),
            ],
            || assert_eq!(KeycloakConfig::from_env(), None),
        );
    }

    #[test]
    fn test_from_env_reads_every_variable() {
        temp_env::with_vars(
            [
                (
                    "KEYCLOAK_REALM_URL",
                    Some("http://keycloak:8080/realms/mairie360/"),
                ),
                ("KEYCLOAK_CLIENT_ID", Some("core-api")),
                ("KEYCLOAK_CLIENT_SECRET", Some("s3cret")),
                ("KEYCLOAK_ISSUER", Some(REALM)),
                ("KEYCLOAK_AUDIENCE", Some(" mairie360-apis, core-api,, ")),
            ],
            || {
                let config = KeycloakConfig::from_env().expect("Keycloak should be enabled");
                assert_eq!(config.realm_url(), "http://keycloak:8080/realms/mairie360");
                assert_eq!(config.issuer(), REALM);
                assert_eq!(config.client_id(), "core-api");
                assert_eq!(config.client_secret(), Some("s3cret"));
                assert_eq!(config.audiences(), ["mairie360-apis", "core-api"]);
            },
        );
    }

    #[test]
    fn test_from_env_defaults_issuer_and_audience() {
        temp_env::with_vars(
            [
                ("KEYCLOAK_REALM_URL", Some(REALM)),
                ("KEYCLOAK_CLIENT_ID", Some("core-api")),
                ("KEYCLOAK_CLIENT_SECRET", Some("")),
                ("KEYCLOAK_ISSUER", None),
                ("KEYCLOAK_AUDIENCE", None),
            ],
            || {
                let config = KeycloakConfig::from_env().expect("Keycloak should be enabled");
                assert_eq!(config.issuer(), REALM);
                assert_eq!(config.client_secret(), None);
                assert_eq!(config.audiences(), ["core-api"]);
            },
        );
    }

    #[test]
    fn test_new_normalises_urls_and_empty_values() {
        let config = KeycloakConfig::new(&format!("{REALM}/"), Some(""), "core-api", Some(""));
        assert_eq!(config.realm_url(), REALM);
        assert_eq!(config.issuer(), REALM);
        assert_eq!(config.client_secret(), None);
        assert_eq!(
            config.token_endpoint(),
            format!("{REALM}/protocol/openid-connect/token")
        );
        assert_eq!(
            config.jwks_uri(),
            format!("{REALM}/protocol/openid-connect/certs")
        );
    }

    #[test]
    fn test_with_audiences_ignores_empty_lists() {
        let config = KeycloakConfig::new(REALM, None, "core-api", None);
        assert_eq!(
            config.clone().with_audiences(&["", "  "]).audiences(),
            ["core-api"]
        );
        assert_eq!(
            config.with_audiences(&["apis", " bff-user "]).audiences(),
            ["apis", "bff-user"]
        );
    }
}

#[cfg(test)]
mod error_tests {
    use super::*;

    #[test]
    fn test_keycloak_error_status_codes() {
        assert_eq!(
            KeycloakError::InvalidToken.status_code(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            KeycloakError::ExpiredToken.status_code(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            KeycloakError::EmailNotVerified.status_code(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            KeycloakError::Unavailable.status_code(),
            StatusCode::BAD_GATEWAY
        );
        let response = KeycloakError::Unavailable.error_response();
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    }

    #[test]
    fn test_keycloak_errors_map_to_jwt_check_errors() {
        assert_eq!(
            JWTCheckError::from(KeycloakError::InvalidToken),
            JWTCheckError::InvalidToken
        );
        assert_eq!(
            JWTCheckError::from(KeycloakError::ExpiredToken),
            JWTCheckError::ExpiredToken
        );
        assert_eq!(
            JWTCheckError::from(KeycloakError::EmailNotVerified),
            JWTCheckError::EmailNotVerified
        );
        assert_eq!(
            JWTCheckError::from(KeycloakError::Unavailable),
            JWTCheckError::IdentityProviderUnavailable
        );
        assert_eq!(
            JWTCheckError::EmailNotVerified.status_code(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            JWTCheckError::IdentityProviderUnavailable.status_code(),
            StatusCode::BAD_GATEWAY
        );
    }
}

#[cfg(test)]
mod verifier_tests {
    use super::*;

    #[tokio::test]
    async fn test_valid_access_token_returns_identity() {
        let mock = KeycloakMock::start();

        let identity = mock.verifier().verify(&mock.access_token(EMAIL)).await;

        assert_eq!(
            identity,
            Ok(KeycloakIdentity {
                subject: SUBJECT.to_string(),
                email: EMAIL.to_string(),
                roles: vec!["User".to_string()],
            })
        );
    }

    #[tokio::test]
    async fn test_email_is_trimmed_and_roles_default_to_empty() {
        let mock = KeycloakMock::start();
        let token = token_with(&mock, |claims| {
            claims["email"] = json!(format!("  {EMAIL} "));
            claims.as_object_mut().unwrap().remove("realm_access");
        });

        let identity = mock.verifier().verify(&token).await.unwrap();

        assert_eq!(identity.email, EMAIL);
        assert!(identity.roles.is_empty());
    }

    #[tokio::test]
    async fn test_key_set_is_cached_between_verifications() {
        let mock = KeycloakMock::start();
        let verifier = mock.verifier();

        verifier.verify(&mock.access_token(EMAIL)).await.unwrap();
        verifier.verify(&mock.access_token(EMAIL)).await.unwrap();

        assert_eq!(mock.jwks_hits(), 1);
    }

    #[tokio::test]
    async fn test_rotated_key_is_fetched_once_then_cached() {
        let mock = KeycloakMock::start();
        let verifier = mock.verifier();
        verifier.verify(&mock.access_token(EMAIL)).await.unwrap();
        mock.publish_keys(&[TestKey::A, TestKey::B]);
        let claims = access_token_claims(mock.realm_url(), EMAIL);
        let token = sign(&claims, TestKey::B, TestKey::B.kid());

        assert!(verifier.verify(&token).await.is_ok());
        assert!(verifier.verify(&token).await.is_ok());

        assert_eq!(mock.jwks_hits(), 2);
    }

    #[tokio::test]
    async fn test_unknown_key_is_invalid() {
        let mock = KeycloakMock::start();
        let claims = access_token_claims(mock.realm_url(), EMAIL);
        let token = sign(&claims, TestKey::B, TestKey::B.kid());

        let result = mock.verifier().verify(&token).await;

        assert_eq!(result, Err(KeycloakError::InvalidToken));
    }

    #[tokio::test]
    async fn test_signature_by_another_key_under_a_known_kid_is_invalid() {
        let mock = KeycloakMock::start();
        let claims = access_token_claims(mock.realm_url(), EMAIL);
        let token = sign(&claims, TestKey::B, TestKey::A.kid());

        let result = mock.verifier().verify(&token).await;

        assert_eq!(result, Err(KeycloakError::InvalidToken));
    }

    #[tokio::test]
    async fn test_tampered_token_is_invalid() {
        let mock = KeycloakMock::start();
        let token = mock.access_token(EMAIL);
        let (head, signature) = token.rsplit_once('.').unwrap();
        let tampered = format!("{head}.{}", signature.chars().rev().collect::<String>());

        let result = mock.verifier().verify(&tampered).await;

        assert_eq!(result, Err(KeycloakError::InvalidToken));
    }

    #[tokio::test]
    async fn test_hmac_token_is_refused_even_with_a_kid() {
        let mock = KeycloakMock::start();
        let header = Header {
            kid: Some(TestKey::A.kid().to_string()),
            ..Header::default()
        };
        let claims = access_token_claims(mock.realm_url(), EMAIL);
        let token = encode(&header, &claims, &EncodingKey::from_secret(b"secret")).unwrap();

        let result = mock.verifier().verify(&token).await;

        assert_eq!(result, Err(KeycloakError::InvalidToken));
        assert_eq!(mock.jwks_hits(), 0, "the key set must not even be fetched");
    }

    #[tokio::test]
    async fn test_token_without_kid_is_invalid() {
        let mock = KeycloakMock::start();
        let header = Header::new(jsonwebtoken::Algorithm::RS256);
        let claims = access_token_claims(mock.realm_url(), EMAIL);
        let token = encode(&header, &claims, &TestKey::A.encoding_key()).unwrap();

        let result = mock.verifier().verify(&token).await;

        assert_eq!(result, Err(KeycloakError::InvalidToken));
    }

    #[tokio::test]
    async fn test_malformed_token_is_invalid() {
        let mock = KeycloakMock::start();

        assert_eq!(
            mock.verifier().verify("not.a.token").await,
            Err(KeycloakError::InvalidToken)
        );
        assert_eq!(
            mock.verifier().verify("").await,
            Err(KeycloakError::InvalidToken)
        );
    }

    #[tokio::test]
    async fn test_expired_token_is_reported_as_expired() {
        let mock = KeycloakMock::start();
        let token = token_with(&mock, |claims| {
            claims["exp"] = json!(get_current_timestamp() - 300);
        });

        let result = mock.verifier().verify(&token).await;

        assert_eq!(result, Err(KeycloakError::ExpiredToken));
    }

    #[tokio::test]
    async fn test_token_from_another_issuer_is_invalid() {
        let mock = KeycloakMock::start();
        let token = token_with(&mock, |claims| {
            claims["iss"] = json!("https://auth.other-town.fr/realms/mairie360");
        });

        let result = mock.verifier().verify(&token).await;

        assert_eq!(result, Err(KeycloakError::InvalidToken));
    }

    #[tokio::test]
    async fn test_public_issuer_can_differ_from_the_realm_url() {
        let mock = KeycloakMock::start();
        let public_issuer = "https://auth.mairie360.fr/realms/mairie360";
        let config = KeycloakConfig::new(mock.realm_url(), Some(public_issuer), CLIENT_ID, None);
        let verifier = mairie360_api_lib::keycloak::KeycloakTokenVerifier::new(config);
        let token = token_with(&mock, |claims| claims["iss"] = json!(public_issuer));

        assert!(verifier.verify(&token).await.is_ok());
        assert_eq!(
            verifier.verify(&mock.access_token(EMAIL)).await,
            Err(KeycloakError::InvalidToken),
            "a token issued under the realm URL must be refused once a public issuer is set"
        );
    }

    #[tokio::test]
    async fn test_missing_required_claims_are_invalid() {
        let mock = KeycloakMock::start();
        for claim in ["exp", "iss", "sub"] {
            let token = token_with(&mock, |claims| {
                claims.as_object_mut().unwrap().remove(claim);
            });
            assert_eq!(
                mock.verifier().verify(&token).await,
                Err(KeycloakError::InvalidToken),
                "a token without `{claim}` must be refused"
            );
        }
    }

    #[tokio::test]
    async fn test_id_and_refresh_tokens_are_not_access_tokens() {
        let mock = KeycloakMock::start();
        for typ in ["ID", "Refresh", "Offline"] {
            let token = token_with(&mock, |claims| claims["typ"] = json!(typ));
            assert_eq!(
                mock.verifier().verify(&token).await,
                Err(KeycloakError::InvalidToken),
                "a `{typ}` token must be refused"
            );
        }
    }

    #[tokio::test]
    async fn test_token_without_typ_is_accepted() {
        let mock = KeycloakMock::start();
        let token = token_with(&mock, |claims| {
            claims.as_object_mut().unwrap().remove("typ");
        });

        assert!(mock.verifier().verify(&token).await.is_ok());
    }

    #[tokio::test]
    async fn test_token_requested_by_another_client_is_invalid() {
        let mock = KeycloakMock::start();
        let token = token_with(&mock, |claims| claims["azp"] = json!("other-client"));

        let result = mock.verifier().verify(&token).await;

        assert_eq!(result, Err(KeycloakError::InvalidToken));
    }

    #[tokio::test]
    async fn test_client_listed_in_aud_is_accepted_whatever_the_azp() {
        let mock = KeycloakMock::start();
        let token = token_with(&mock, |claims| {
            claims["azp"] = json!("login-front");
            claims["aud"] = json!([CLIENT_ID, "account"]);
        });

        assert!(mock.verifier().verify(&token).await.is_ok());
    }

    #[tokio::test]
    async fn test_token_without_aud_is_accepted_on_azp() {
        let mock = KeycloakMock::start();
        let token = token_with(&mock, |claims| {
            claims.as_object_mut().unwrap().remove("aud");
        });

        assert!(mock.verifier().verify(&token).await.is_ok());
    }

    #[tokio::test]
    async fn test_configured_audiences_replace_the_client_id() {
        let mock = KeycloakMock::start();
        let config = mock.config().with_audiences(&["mairie360-apis"]);
        let verifier = mairie360_api_lib::keycloak::KeycloakTokenVerifier::new(config);
        let for_apis = token_with(&mock, |claims| {
            claims["azp"] = json!("login-front");
            claims["aud"] = json!("mairie360-apis");
        });

        assert!(verifier.verify(&for_apis).await.is_ok());
        assert_eq!(
            verifier.verify(&mock.access_token(EMAIL)).await,
            Err(KeycloakError::InvalidToken),
            "a token only carrying the client id must be refused once audiences are configured"
        );
    }

    #[tokio::test]
    async fn test_unverified_or_missing_email_is_refused() {
        let mock = KeycloakMock::start();
        let unverified = token_with(&mock, |claims| claims["email_verified"] = json!(false));
        let missing = token_with(&mock, |claims| {
            claims.as_object_mut().unwrap().remove("email");
            claims.as_object_mut().unwrap().remove("email_verified");
        });
        let blank = token_with(&mock, |claims| claims["email"] = json!("   "));

        for token in [unverified, missing, blank] {
            assert_eq!(
                mock.verifier().verify(&token).await,
                Err(KeycloakError::EmailNotVerified)
            );
        }
    }

    #[tokio::test]
    async fn test_unreachable_realm_is_unavailable() {
        let config = KeycloakConfig::new("http://127.0.0.1:9/realms/down", None, CLIENT_ID, None);
        let verifier = mairie360_api_lib::keycloak::KeycloakTokenVerifier::new(config);
        let claims = access_token_claims("http://127.0.0.1:9/realms/down", EMAIL);
        let token = sign(&claims, TestKey::A, TestKey::A.kid());

        let result = verifier.verify(&token).await;

        assert_eq!(result, Err(KeycloakError::Unavailable));
    }

    #[tokio::test]
    async fn test_unreadable_key_set_is_unavailable() {
        let mock = KeycloakMock::start();
        // The realm URL points at a route that exists but does not serve a JWKS.
        let bogus_realm = format!("{}/protocol/openid-connect", mock.realm_url());
        let config = KeycloakConfig::new(&bogus_realm, Some(mock.realm_url()), CLIENT_ID, None);
        let verifier = mairie360_api_lib::keycloak::KeycloakTokenVerifier::new(config);

        let result = verifier.verify(&mock.access_token(EMAIL)).await;

        assert_eq!(result, Err(KeycloakError::Unavailable));
    }
}
