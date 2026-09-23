use super::{KeycloakConfig, KeycloakError, KeycloakIdentity};
use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{decode, decode_header, errors::ErrorKind, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use std::time::Duration;
use tokio::sync::RwLock;

/// Asymmetric algorithms accepted for Keycloak tokens. HMAC is refused so that a token can
/// never be verified with a secret published in the realm's key set, and so that the historical
/// `JWT_SECRET` tokens are never mistaken for Keycloak ones.
const ACCEPTED_ALGORITHMS: [Algorithm; 9] = [
    Algorithm::RS256,
    Algorithm::RS384,
    Algorithm::RS512,
    Algorithm::PS256,
    Algorithm::PS384,
    Algorithm::PS512,
    Algorithm::ES256,
    Algorithm::ES384,
    Algorithm::EdDSA,
];

/// Clock skew tolerated between Keycloak and the API when checking `exp`, in seconds.
const CLOCK_SKEW_LEEWAY: u64 = 30;

/// Timeout of every HTTP call to Keycloak.
const HTTP_TIMEOUT: Duration = Duration::from_secs(10);

/// `typ` claim Keycloak writes into access tokens; ID tokens carry `ID` and refresh tokens
/// `Refresh`, neither of which grants access to an API.
const ACCESS_TOKEN_TYPE: &str = "Bearer";

/// `aud` claim: a single audience or a list, as allowed by RFC 7519.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Audience {
    Single(String),
    Several(Vec<String>),
}

impl Audience {
    fn contains(&self, audience: &str) -> bool {
        match self {
            Self::Single(value) => value == audience,
            Self::Several(values) => values.iter().any(|value| value == audience),
        }
    }
}

#[derive(Debug, Deserialize)]
struct RealmAccess {
    #[serde(default)]
    roles: Vec<String>,
}

/// Claims read from a Keycloak access token; `iss` and `exp` are checked by `jsonwebtoken`.
#[derive(Debug, Deserialize)]
struct AccessTokenClaims {
    sub: String,
    typ: Option<String>,
    azp: Option<String>,
    aud: Option<Audience>,
    email: Option<String>,
    #[serde(default)]
    email_verified: bool,
    realm_access: Option<RealmAccess>,
}

/// Verifies Keycloak access tokens against the realm's published keys, which are cached and
/// fetched again once when a token announces an unknown key (key rotation).
///
/// Held by `AppState` and shared by every request: the key cache is behind an async lock.
pub struct KeycloakTokenVerifier {
    config: KeycloakConfig,
    http: reqwest::Client,
    jwks: RwLock<Option<JwkSet>>,
}

impl KeycloakTokenVerifier {
    /// # Panics
    ///
    /// Panics if the HTTP client cannot be built (TLS backend initialisation failure), which
    /// only happens on a broken host and must stop the API at startup.
    #[must_use]
    pub fn new(config: KeycloakConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(HTTP_TIMEOUT)
            .build()
            .expect("Failed to build the Keycloak HTTP client");
        Self {
            config,
            http,
            jwks: RwLock::new(None),
        }
    }

    #[must_use]
    pub const fn config(&self) -> &KeycloakConfig {
        &self.config
    }

    /// Verifies `token` and returns the identity it asserts.
    ///
    /// Checks, in order: an accepted asymmetric algorithm and a `kid` header, the signature
    /// against the realm key of that id, `iss` equal to the configured issuer, `exp` (with a
    /// short clock-skew leeway), the `typ` claim (`Bearer`, i.e. an access token), the audience
    /// (one of the configured audiences in `aud`, or as `azp`), and finally a verified e-mail.
    ///
    /// # Errors
    ///
    /// - [`KeycloakError::ExpiredToken`] if the token is valid but expired;
    /// - [`KeycloakError::InvalidToken`] if any other check on the token itself fails;
    /// - [`KeycloakError::EmailNotVerified`] if the token has no verified e-mail;
    /// - [`KeycloakError::Unavailable`] if the realm keys cannot be fetched.
    pub async fn verify(&self, token: &str) -> Result<KeycloakIdentity, KeycloakError> {
        let header = decode_header(token).map_err(|_| KeycloakError::InvalidToken)?;
        if !ACCEPTED_ALGORITHMS.contains(&header.alg) {
            eprintln!(
                "Keycloak token rejected: algorithm {:?} is not accepted.",
                header.alg
            );
            return Err(KeycloakError::InvalidToken);
        }
        let kid = header.kid.ok_or_else(|| {
            eprintln!("Keycloak token rejected: no `kid` header.");
            KeycloakError::InvalidToken
        })?;
        let key = self.decoding_key(&kid).await?;
        if header.alg.family() != key.family() {
            eprintln!("Keycloak token rejected: algorithm does not match the key `{kid}`.");
            return Err(KeycloakError::InvalidToken);
        }

        let mut validation = Validation::new(header.alg);
        validation.leeway = CLOCK_SKEW_LEEWAY;
        validation.set_issuer(&[self.config.issuer()]);
        // The audience is checked below: Keycloak only lists a client in `aud` when the realm
        // has an audience mapper, otherwise the requesting client is only visible as `azp`.
        validation.validate_aud = false;
        validation.set_required_spec_claims(&["exp", "iss", "sub"]);

        let claims = decode::<AccessTokenClaims>(token, &key, &validation)
            .map_err(|e| {
                if matches!(e.kind(), ErrorKind::ExpiredSignature) {
                    KeycloakError::ExpiredToken
                } else {
                    eprintln!("Keycloak token rejected: {e}");
                    KeycloakError::InvalidToken
                }
            })?
            .claims;

        if let Some(typ) = claims.typ.as_deref() {
            if typ != ACCESS_TOKEN_TYPE {
                eprintln!("Keycloak token rejected: `{typ}` token used as an access token.");
                return Err(KeycloakError::InvalidToken);
            }
        }
        if !self.is_for_this_api(&claims) {
            eprintln!(
                "Keycloak token rejected: issued for another audience (azp = {:?}).",
                claims.azp
            );
            return Err(KeycloakError::InvalidToken);
        }

        match claims.email {
            Some(email) if claims.email_verified && !email.trim().is_empty() => {
                Ok(KeycloakIdentity {
                    subject: claims.sub,
                    email: email.trim().to_string(),
                    roles: claims
                        .realm_access
                        .map(|access| access.roles)
                        .unwrap_or_default(),
                })
            }
            _ => Err(KeycloakError::EmailNotVerified),
        }
    }

    fn is_for_this_api(&self, claims: &AccessTokenClaims) -> bool {
        self.config.audiences().iter().any(|audience| {
            claims.azp.as_deref() == Some(audience.as_str())
                || claims
                    .aud
                    .as_ref()
                    .is_some_and(|aud| aud.contains(audience))
        })
    }

    /// Returns the realm key `kid`, fetching the key set again once if it is unknown (Keycloak
    /// rotated its keys since the last fetch).
    async fn decoding_key(&self, kid: &str) -> Result<DecodingKey, KeycloakError> {
        if let Some(key) = self.cached_key(kid).await {
            return key;
        }
        let jwks = self.fetch_jwks().await?;
        let key = jwks.find(kid).map(DecodingKey::from_jwk);
        *self.jwks.write().await = Some(jwks);
        key.map_or_else(
            || {
                eprintln!("Keycloak token rejected: unknown key `{kid}`.");
                Err(KeycloakError::InvalidToken)
            },
            |key| key.map_err(|_| KeycloakError::InvalidToken),
        )
    }

    async fn cached_key(&self, kid: &str) -> Option<Result<DecodingKey, KeycloakError>> {
        let jwks = self.jwks.read().await;
        jwks.as_ref()?
            .find(kid)
            .map(|jwk| DecodingKey::from_jwk(jwk).map_err(|_| KeycloakError::InvalidToken))
    }

    async fn fetch_jwks(&self) -> Result<JwkSet, KeycloakError> {
        let response = self
            .http
            .get(self.config.jwks_uri())
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(|e| {
                eprintln!("Keycloak key set unreachable: {e}");
                KeycloakError::Unavailable
            })?;
        response.json().await.map_err(|e| {
            eprintln!("Unreadable Keycloak key set: {e}");
            KeycloakError::Unavailable
        })
    }
}
