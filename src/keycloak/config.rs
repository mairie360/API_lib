use crate::env_manager::get_env_var;

/// Connection settings of the Keycloak realm whose tokens the API accepts.
///
/// Keycloak is optional during the transition from the password login: when the realm URL or
/// the client id is missing, [`KeycloakConfig::from_env`] returns `None`, the middlewares only
/// accept the JWTs signed with `JWT_SECRET`, and any Keycloak-shaped token is refused as invalid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeycloakConfig {
    realm_url: String,
    issuer: String,
    client_id: String,
    client_secret: Option<String>,
    audiences: Vec<String>,
}

impl KeycloakConfig {
    /// Builds a configuration from its raw parts.
    ///
    /// `realm_url` is the URL the API uses to reach the realm (e.g.
    /// `http://keycloak:8080/realms/mairie360`). `issuer` is the `iss` claim Keycloak writes into
    /// its tokens, which follows Keycloak's public hostname and can therefore differ from
    /// `realm_url` inside a cluster; it defaults to `realm_url`. Trailing slashes are ignored.
    /// The accepted audiences default to `client_id` (see [`KeycloakConfig::with_audiences`]).
    #[must_use]
    pub fn new(
        realm_url: &str,
        issuer: Option<&str>,
        client_id: &str,
        client_secret: Option<&str>,
    ) -> Self {
        let realm_url = realm_url.trim_end_matches('/').to_string();
        let issuer = issuer
            .map(|issuer| issuer.trim_end_matches('/').to_string())
            .filter(|issuer| !issuer.is_empty())
            .unwrap_or_else(|| realm_url.clone());
        Self {
            realm_url,
            issuer,
            client_id: client_id.to_string(),
            client_secret: client_secret
                .filter(|secret| !secret.is_empty())
                .map(ToString::to_string),
            audiences: vec![client_id.to_string()],
        }
    }

    /// Replaces the audiences a token may be issued for (the `aud` claim, or the `azp` claim
    /// for a token requested by that client). Empty values are ignored; an empty list falls back
    /// to the client id.
    #[must_use]
    pub fn with_audiences(mut self, audiences: &[&str]) -> Self {
        let audiences: Vec<String> = audiences
            .iter()
            .map(|audience| audience.trim())
            .filter(|audience| !audience.is_empty())
            .map(ToString::to_string)
            .collect();
        if !audiences.is_empty() {
            self.audiences = audiences;
        }
        self
    }

    /// Reads `KEYCLOAK_REALM_URL` and `KEYCLOAK_CLIENT_ID` (both required to enable Keycloak),
    /// `KEYCLOAK_CLIENT_SECRET` (optional, confidential client), `KEYCLOAK_ISSUER` (optional,
    /// defaults to the realm URL) and `KEYCLOAK_AUDIENCE` (optional, comma-separated list of
    /// accepted audiences, defaults to the client id).
    ///
    /// Returns `None`, which disables Keycloak tokens, when the realm URL or the client id is
    /// missing or empty; a half-set pair is logged since it is most likely a mistake.
    #[must_use]
    pub fn from_env() -> Option<Self> {
        let realm_url = get_env_var("KEYCLOAK_REALM_URL").filter(|value| !value.trim().is_empty());
        let client_id = get_env_var("KEYCLOAK_CLIENT_ID").filter(|value| !value.trim().is_empty());
        match (realm_url, client_id) {
            (Some(realm_url), Some(client_id)) => {
                let config = Self::new(
                    realm_url.trim(),
                    get_env_var("KEYCLOAK_ISSUER").as_deref(),
                    client_id.trim(),
                    get_env_var("KEYCLOAK_CLIENT_SECRET").as_deref(),
                );
                let audiences = get_env_var("KEYCLOAK_AUDIENCE").unwrap_or_default();
                let audiences: Vec<&str> = audiences.split(',').collect();
                Some(config.with_audiences(&audiences))
            }
            (None, None) => None,
            _ => {
                eprintln!(
                    "Keycloak tokens disabled: KEYCLOAK_REALM_URL and KEYCLOAK_CLIENT_ID must both be set."
                );
                None
            }
        }
    }

    #[must_use]
    pub fn realm_url(&self) -> &str {
        &self.realm_url
    }

    /// Value the `iss` claim of every accepted token must carry.
    #[must_use]
    pub fn issuer(&self) -> &str {
        &self.issuer
    }

    #[must_use]
    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    #[must_use]
    pub fn client_secret(&self) -> Option<&str> {
        self.client_secret.as_deref()
    }

    /// Audiences a token may be issued for; at least one of them must appear in its `aud` claim
    /// or be its `azp` (the client that requested it).
    #[must_use]
    pub fn audiences(&self) -> &[String] {
        &self.audiences
    }

    /// OIDC token endpoint of the realm.
    #[must_use]
    pub fn token_endpoint(&self) -> String {
        format!("{}/protocol/openid-connect/token", self.realm_url)
    }

    /// JSON Web Key Set holding the realm's public signing keys.
    #[must_use]
    pub fn jwks_uri(&self) -> String {
        format!("{}/protocol/openid-connect/certs", self.realm_url)
    }
}
