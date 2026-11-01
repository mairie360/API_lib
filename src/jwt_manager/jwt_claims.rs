use serde::{Deserialize, Serialize};

/// Claims of the tokens issued by [`generate_jwt`](super::generate_jwt).
#[derive(Serialize, Deserialize)]
pub struct Claims {
    sub: String,
    pub role: String,
    exp: usize,
    /// Id of the session the token belongs to (MAIR-264). Absent from tokens issued without a
    /// session: they are still accepted, but cannot be revoked before they expire.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sid: Option<String>,
    /// Issuer, [`SESSION_JWT_ISSUER`](super::SESSION_JWT_ISSUER) on the tokens Core signs with
    /// its Ed25519 key (MAIR-680); absent from the historical `JWT_SECRET` tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    iss: Option<String>,
    /// Audience, [`SESSION_JWT_AUDIENCE`](super::SESSION_JWT_AUDIENCE) on the tokens Core signs
    /// with its Ed25519 key (MAIR-680); absent from the historical `JWT_SECRET` tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    aud: Option<String>,
}

impl Claims {
    #[must_use]
    pub fn new(user_id: &str, role: &str, expiration: usize) -> Self {
        Self {
            sub: user_id.to_string(),
            role: role.to_string(),
            exp: expiration,
            sid: None,
            iss: None,
            aud: None,
        }
    }

    /// Sets the `iss` and `aud` claims of the tokens signed with Core's Ed25519 key.
    #[must_use]
    pub fn with_issuer_and_audience(mut self, issuer: &str, audience: &str) -> Self {
        self.iss = Some(issuer.to_string());
        self.aud = Some(audience.to_string());
        self
    }

    /// Binds the claims to the session `session_id`.
    #[must_use]
    pub fn with_session_id(mut self, session_id: &str) -> Self {
        self.sid = Some(session_id.to_string());
        self
    }

    #[must_use]
    pub fn user_id(&self) -> &str {
        &self.sub
    }

    #[must_use]
    pub const fn expiration(&self) -> usize {
        self.exp
    }

    #[must_use]
    pub fn role(&self) -> &str {
        &self.role
    }

    /// Issuer of the token, if any.
    #[must_use]
    pub fn issuer(&self) -> Option<&str> {
        self.iss.as_deref()
    }

    /// Audience of the token, if any.
    #[must_use]
    pub fn audience(&self) -> Option<&str> {
        self.aud.as_deref()
    }

    /// Id of the session the token belongs to, if any.
    #[must_use]
    pub fn session_id(&self) -> Option<&str> {
        self.sid.as_deref()
    }
}

impl std::fmt::Display for Claims {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Claims {{ sub: {}, role: {}, exp: {}, sid: {} }}",
            self.sub,
            self.role,
            self.exp,
            self.sid.as_deref().unwrap_or("none")
        )
    }
}
