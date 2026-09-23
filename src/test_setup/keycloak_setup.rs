//! Minimal Keycloak realm for tests.
//!
//! A JWKS endpoint served by a real HTTP server on a random local port, and access tokens
//! signed by throwaway RSA keys, so the verifier and the middlewares can be exercised without a
//! Keycloak instance. Usable by the APIs' own tests.

use crate::keycloak::{KeycloakConfig, KeycloakTokenVerifier};
use actix_web::{web, App, HttpResponse, HttpServer};
use jsonwebtoken::{encode, get_current_timestamp, Algorithm, EncodingKey, Header};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// Client id of the API in the fake realm (the `azp` of the tokens it issues).
pub const CLIENT_ID: &str = "mairie360-api";
/// Keycloak user id (`sub`) of the tokens built by [`access_token_claims`].
pub const SUBJECT: &str = "f3b2c1d0-7a6e-4c5b-9d8e-1a2b3c4d5e6f";

/// Test-only RSA keys (PKCS#1 DER), generated for this test suite and never used elsewhere.
const KEY_A_DER: &[u8] = include_bytes!("fixtures/keycloak_test_key_a.der");
const KEY_B_DER: &[u8] = include_bytes!("fixtures/keycloak_test_key_b.der");
const KEY_A_MODULUS: &str = "2DWM8YrcNHL-EU2qgY-ipXCOAeKrAMJAMGMgiv7dDliC7MaNqIU0lPZzxQtkv6Uqq3QKQnHyxdR_0GQi5pMXFLA9NBoHsr3OuPnyIXd6PxFdqbooGnLLLaPiZScst2Q2_H2mA43jAviVALwQ0qRTRrq56WyVnP3JNARDRVDm0iss6JJVTjkrA9BjHrp-vCLtUzp7QEVZvdPUJpGpyQq-BlTz3IGP2aP2YmCgiocLaWYG1607oCXnIYsepzOgZg5IjdkMqINmccb39dCkoIIFtDfofnRas-_CM7tCpbrTsTUC1iYlYkFW2ccsQZ-5hQBfy4Edjfq_TqC0K4gjhTJB1Q";
const KEY_B_MODULUS: &str = "vZbYiGZrDbCbflUE9rmz8uxqQ41uXU3BVSM8sP0QopsgKk8wu2dJDUjh71Q8ahqG-wVmzeIQQJ6hETnobgYtcfXnBAfaHcbMEkRM-I784uboiU7joDsO8RHg46IpeALMoPzwRDltSOVRKBA9jPhg6RlkbJRQaU6lpfDr-WdvrtT_YEZ1zA4SMKl_gIUBqFk_-xhYDQJ6yp33ajJ4NJfSyzM-v_DSaOBQs61bFBN8F-cV8fQZTLK4lmow_kmT1ZRY7N1IesWM8AORUv2RMX8LszlANYDZzBQTVsXKnxPsTEMDCJtG8PQgffGmACl75TRX6oTww3QxBesTp18AQy5ATw";

/// One of the two test signing keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestKey {
    A,
    B,
}

impl TestKey {
    #[must_use]
    pub const fn kid(self) -> &'static str {
        match self {
            Self::A => "test-key-a",
            Self::B => "test-key-b",
        }
    }

    const fn der(self) -> &'static [u8] {
        match self {
            Self::A => KEY_A_DER,
            Self::B => KEY_B_DER,
        }
    }

    /// Private key of this test key, to sign tokens with a custom header (see [`sign`] for the
    /// usual case).
    #[must_use]
    pub fn encoding_key(self) -> EncodingKey {
        EncodingKey::from_rsa_der(self.der())
    }

    const fn modulus(self) -> &'static str {
        match self {
            Self::A => KEY_A_MODULUS,
            Self::B => KEY_B_MODULUS,
        }
    }

    /// Public JWK of this key, as Keycloak publishes it on its certs endpoint.
    #[must_use]
    pub fn jwk(self) -> Value {
        json!({
            "kid": self.kid(),
            "kty": "RSA",
            "alg": "RS256",
            "use": "sig",
            "n": self.modulus(),
            "e": "AQAB"
        })
    }
}

/// Signs `claims` (RS256) with `key`, announcing `kid` in the header (normally `key.kid()`).
///
/// # Panics
///
/// Panics if `claims` cannot be serialised or signed, which cannot happen with a JSON value
/// and the embedded test keys.
#[must_use]
pub fn sign(claims: &Value, key: TestKey, kid: &str) -> String {
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(kid.to_string());
    encode(&header, claims, &key.encoding_key()).expect("Failed to sign the test Keycloak token")
}

/// Claims of a valid access token for `email`, issued by `issuer` to [`CLIENT_ID`], the way
/// Keycloak builds them by default (`typ` Bearer, `aud` account, the client only in `azp`).
#[must_use]
pub fn access_token_claims(issuer: &str, email: &str) -> Value {
    let now = get_current_timestamp();
    json!({
        "iss": issuer,
        "aud": "account",
        "sub": SUBJECT,
        "typ": "Bearer",
        "azp": CLIENT_ID,
        "iat": now,
        "exp": now + 300,
        "email": email,
        "email_verified": true,
        "realm_access": { "roles": ["User"] },
        "scope": "openid email profile"
    })
}

struct MockState {
    jwks: Mutex<Value>,
    jwks_hits: AtomicUsize,
}

/// A running fake Keycloak realm. The server stops when the test's runtime shuts down.
pub struct KeycloakMock {
    realm_url: String,
    state: Arc<MockState>,
}

impl KeycloakMock {
    /// Starts, on the current Tokio runtime, a realm publishing [`TestKey::A`] only.
    ///
    /// # Panics
    ///
    /// Panics if no local port can be bound.
    #[must_use]
    pub fn start() -> Self {
        let state = Arc::new(MockState {
            jwks: Mutex::new(json!({ "keys": [TestKey::A.jwk()] })),
            jwks_hits: AtomicUsize::new(0),
        });
        let data = web::Data::from(state.clone());
        let server = HttpServer::new(move || {
            App::new().app_data(data.clone()).route(
                "/realms/test/protocol/openid-connect/certs",
                web::get().to(certs),
            )
        })
        .workers(1)
        .disable_signals()
        .bind(("127.0.0.1", 0))
        .expect("Failed to bind the Keycloak mock");
        let port = server.addrs()[0].port();
        tokio::spawn(server.run());
        Self {
            realm_url: format!("http://127.0.0.1:{port}/realms/test"),
            state,
        }
    }

    /// Realm URL, also the issuer of the tokens it signs.
    #[must_use]
    pub fn realm_url(&self) -> &str {
        &self.realm_url
    }

    /// Configuration of [`CLIENT_ID`] pointing at this realm.
    #[must_use]
    pub fn config(&self) -> KeycloakConfig {
        KeycloakConfig::new(&self.realm_url, None, CLIENT_ID, None)
    }

    #[must_use]
    pub fn verifier(&self) -> KeycloakTokenVerifier {
        KeycloakTokenVerifier::new(self.config())
    }

    /// A valid access token for `email`, signed by [`TestKey::A`].
    #[must_use]
    pub fn access_token(&self, email: &str) -> String {
        sign(
            &access_token_claims(&self.realm_url, email),
            TestKey::A,
            TestKey::A.kid(),
        )
    }

    /// Replaces the published key set.
    ///
    /// # Panics
    ///
    /// Panics if the key set lock is poisoned.
    pub fn publish_keys(&self, keys: &[TestKey]) {
        let keys: Vec<Value> = keys.iter().map(|key| key.jwk()).collect();
        *self
            .state
            .jwks
            .lock()
            .expect("Keycloak mock key set poisoned") = json!({ "keys": keys });
    }

    /// Number of times the key set was downloaded.
    #[must_use]
    pub fn jwks_hits(&self) -> usize {
        self.state.jwks_hits.load(Ordering::SeqCst)
    }
}

async fn certs(state: web::Data<MockState>) -> HttpResponse {
    state.jwks_hits.fetch_add(1, Ordering::SeqCst);
    let jwks = state
        .jwks
        .lock()
        .expect("Keycloak mock key set poisoned")
        .clone();
    HttpResponse::Ok().json(jwks)
}
