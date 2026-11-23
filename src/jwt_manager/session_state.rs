//! State of a session in Redis (MAIR-772): `session:<sid>` lets the JWT middleware accept a token
//! without a database query, and refuse a token replaced by a newer one or a revoked session.
//!
//! `Core_API` is the only writer of the hash of the active JWT ([`publish_session_state`], at every
//! sign-in and refresh) and of the revoked state ([`mark_session_revoked`]). The middleware of every API
//! reads the key and, when it is missing, checks the database (`session_state()`) and writes the state
//! without hash with `SET NX` ([`cache_active_session`]), which never overwrites a revoked state. The key
//! lives as long as the JWT that wrote it (15 minutes at most).

use super::session_revocation::revoke_session;
use crate::redis::error::RedisError;
use crate::redis::redis_interface::Redis;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::Duration;

/// Prefix of the shared (unprefixed) session keys.
pub const SESSION_STATE_KEY_PREFIX: &str = "session:";

/// Longest time to live of a `session:<sid>` key (15 minutes).
pub const MAX_SESSION_STATE_TTL_SECONDS: u64 = 15 * 60;

const LOOKUP_TIMEOUT: Duration = Duration::from_secs(2);

/// Whether the session may still be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Active,
    Revoked,
}

/// Value of `session:<sid>`, stored as JSON. `jwt` is the hex SHA-256 of the active JWT (never the JWT
/// itself), absent when the state was written by a middleware after a database check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jwt: Option<String>,
    pub user_id: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,
    pub state: SessionStatus,
}

impl SessionState {
    /// State of a session whose active JWT is `jwt` (Core, at sign-in and refresh).
    #[must_use]
    pub fn active_with_token(user_id: u64, jwt: &str, ip: Option<&str>) -> Self {
        Self {
            jwt: Some(jwt_hash(jwt)),
            user_id,
            ip: ip.map(str::to_string),
            state: SessionStatus::Active,
        }
    }

    /// Whether `jwt` is the active token of this state: equal hashes compared in constant time, or no
    /// hash recorded (state written by a middleware).
    #[must_use]
    pub fn accepts(&self, jwt: &str) -> bool {
        self.jwt
            .as_deref()
            .is_none_or(|expected| constant_time_eq(expected.as_bytes(), jwt_hash(jwt).as_bytes()))
    }
}

impl std::fmt::Display for SessionState {
    // Never the hash nor the address.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SessionState {{ user_id: {}, state: {:?} }}",
            self.user_id, self.state
        )
    }
}

/// `session:<sid>`.
#[must_use]
pub fn session_state_key(session_id: &str) -> String {
    format!("{SESSION_STATE_KEY_PREFIX}{session_id}")
}

/// Hex SHA-256 of a JWT.
#[must_use]
pub fn jwt_hash(jwt: &str) -> String {
    use std::fmt::Write;
    Sha256::digest(jwt.as_bytes())
        .iter()
        .fold(String::with_capacity(64), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        })
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0_u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn ttl(seconds: u64) -> u64 {
    seconds.clamp(1, MAX_SESSION_STATE_TTL_SECONDS)
}

/// Writes the state of a session whose active JWT was just issued (Core, at sign-in and refresh).
///
/// The new hash replaces the previous one. `ttl_seconds` is the lifetime of that JWT (clamped to 15 minutes).
///
/// # Errors
///
/// The Redis error: the caller must then refuse to hand out the token (a token is always known to Redis).
pub async fn publish_session_state(
    redis: &Redis,
    session_id: &str,
    state: &SessionState,
    ttl_seconds: u64,
) -> Result<(), RedisError> {
    let value = serde_json::to_string(state).map_err(|e| RedisError::Value(e.to_string()))?;
    redis
        .set_ex_shared(&session_state_key(session_id), value, ttl(ttl_seconds))
        .await
}

/// Revokes a session in Redis (Core, at every revocation).
///
/// `session:<sid>` becomes `revoked` (overwritten,
/// never deleted, so a middleware `SET NX` cannot bring the session back) and `revoked:<sid>` is written,
/// both for `ttl_seconds` (clamped to 15 minutes for the state).
///
/// # Errors
///
/// The first Redis error.
pub async fn mark_session_revoked(
    redis: &Redis,
    session_id: &str,
    user_id: u64,
    ttl_seconds: u64,
) -> Result<(), RedisError> {
    let state = SessionState {
        jwt: None,
        user_id,
        ip: None,
        state: SessionStatus::Revoked,
    };
    let value = serde_json::to_string(&state).map_err(|e| RedisError::Value(e.to_string()))?;
    redis
        .set_ex_shared(&session_state_key(session_id), value, ttl(ttl_seconds))
        .await?;
    revoke_session(redis, session_id, ttl_seconds).await
}

/// Reads `session:<sid>` (`None` when absent). A value that does not parse reads as absent, so the
/// middleware checks the database.
///
/// # Errors
///
/// The Redis error, or a timeout after 2 seconds.
pub async fn read_session_state(
    redis: &Redis,
    session_id: &str,
) -> Result<Option<SessionState>, RedisError> {
    let raw = tokio::time::timeout(
        LOOKUP_TIMEOUT,
        redis.get_shared(&session_state_key(session_id)),
    )
    .await
    .map_err(|_| RedisError::Pool("session state lookup timed out".to_string()))??;
    Ok(raw.and_then(|value| serde_json::from_str(&value).ok()))
}

/// Records, after a database check, that a session is active.
///
/// Written without the hash of its JWT, only if the key does not exist (`SET NX`): it never overwrites a revoked state. Returns whether it was written.
///
/// # Errors
///
/// The Redis error.
pub async fn cache_active_session(
    redis: &Redis,
    session_id: &str,
    user_id: u64,
    ip: Option<&str>,
    ttl_seconds: u64,
) -> Result<bool, RedisError> {
    let state = SessionState {
        jwt: None,
        user_id,
        ip: ip.map(str::to_string),
        state: SessionStatus::Active,
    };
    let value = serde_json::to_string(&state).map_err(|e| RedisError::Value(e.to_string()))?;
    redis
        .set_ex_nx_shared(&session_state_key(session_id), value, ttl(ttl_seconds))
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hash_of_the_active_token_accepts_it_and_refuses_another() {
        let state =
            SessionState::active_with_token(7, "header.payload.signature", Some("92.184.10.1"));
        assert!(state.accepts("header.payload.signature"));
        assert!(!state.accepts("header.payload.other"));
        assert_ne!(
            state.jwt.as_deref(),
            Some("header.payload.signature"),
            "never the JWT itself"
        );
    }

    #[test]
    fn a_state_written_after_a_database_check_accepts_any_token_of_the_session() {
        let state = SessionState {
            jwt: None,
            user_id: 7,
            ip: None,
            state: SessionStatus::Active,
        };
        assert!(state.accepts("whatever"));
    }

    #[test]
    fn the_state_round_trips_as_json_and_never_prints_its_hash() {
        let state = SessionState::active_with_token(7, "a.b.c", None);
        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains("\"state\":\"active\""));
        assert_eq!(serde_json::from_str::<SessionState>(&json).unwrap(), state);
        assert!(!state.to_string().contains(state.jwt.as_deref().unwrap()));
    }

    #[test]
    fn the_time_to_live_stays_between_one_second_and_fifteen_minutes() {
        assert_eq!(ttl(0), 1);
        assert_eq!(ttl(3600), MAX_SESSION_STATE_TTL_SECONDS);
        assert_eq!(ttl(120), 120);
    }
}
