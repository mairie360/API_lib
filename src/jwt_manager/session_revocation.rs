//! Revocation list of JWT sessions, shared by every API through Redis (MAIR-264).
//!
//! A token carrying a session id (`sid` claim) is refused once the key `revoked:<sid>` exists.
//! Core API writes the key when a session ends (logout, revocation, archived account, password
//! change) with a time to live equal to the remaining lifetime of the session's tokens, so the
//! list never outgrows the tokens it blocks.
//!
//! Keys are used verbatim: this crate never prefixes Redis keys (not with the API role either),
//! so every API reads the same `revoked:<sid>` key. The Redis ACL grants every API role read
//! access (`EXISTS`) on `revoked:*` and the Core API write access (`SET`).

use crate::redis::error::RedisError;
use crate::redis::redis_interface::Redis;
use std::time::Duration;

/// Prefix of the revocation keys, shared by every API.
pub const REVOKED_SESSION_KEY_PREFIX: &str = "revoked:";

/// Upper bound of the revocation lookup: past it, Redis is treated as unavailable.
const LOOKUP_TIMEOUT: Duration = Duration::from_secs(2);

/// Redis key marking the session `session_id` as revoked: `revoked:<session_id>`.
#[must_use]
pub fn revoked_session_key(session_id: &str) -> String {
    format!("{REVOKED_SESSION_KEY_PREFIX}{session_id}")
}

/// Marks the session `session_id` as revoked for `ttl_seconds` seconds (at least 1).
///
/// Use the remaining lifetime of the session's tokens, at most `JWT_TIMEOUT`: past it, the
/// tokens are expired anyway. Revoking twice only refreshes the TTL.
///
/// # Errors
///
/// Returns a [`RedisError`] when Redis cannot be reached or rejects the write.
pub async fn revoke_session(
    redis: &Redis,
    session_id: &str,
    ttl_seconds: u64,
) -> Result<(), RedisError> {
    redis
        .set_ex(&revoked_session_key(session_id), 1_i32, ttl_seconds.max(1))
        .await
}

/// Whether the session `session_id` was revoked.
///
/// # Errors
///
/// Returns a [`RedisError`] when Redis cannot be reached, rejects the command or does not
/// answer within 2 seconds: callers must then fail closed.
pub async fn is_session_revoked(redis: &Redis, session_id: &str) -> Result<bool, RedisError> {
    tokio::time::timeout(
        LOOKUP_TIMEOUT,
        redis.key_exist(&revoked_session_key(session_id)),
    )
    .await
    .map_err(|_| RedisError::Pool("revocation lookup timed out".to_string()))?
}
