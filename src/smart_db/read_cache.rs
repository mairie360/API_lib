//! Read-through cache of the heavy list routes (MAIR-524), on top of [`SmartDatabase`].
//!
//! A [`ReadCache`] names a family of reads invalidated together (`"calendar"`, `"projects"`,
//! `"directory"`…) and their time to live. Its entries are keyed by the **caller** and a hash of
//! the request parameters, so one user never reads another user's page:
//!
//! ```text
//! read-cache:<namespace>:<generation>:u<user id>:<sha256(params)>   (ReadCache::for_user)
//! read-cache:<namespace>:<generation>:shared:<sha256(params)>       (ReadCache::shared)
//! ```
//!
//! # Invalidation by generation
//!
//! The Redis ACL of the platform grants `GET`, `SET`, `DEL`, `EXISTS` and `EXPIRE` only: no
//! `SCAN`, `KEYS` nor `INCR`, so the entries of a namespace cannot be listed and deleted. Every
//! entry key embeds the namespace's current **generation** instead, read from
//! `read-cache:<namespace>:generation`; [`SmartDatabase::invalidate_read_cache`] (or a write view
//! listing the namespace in [`ApiRequestDto::invalidated_read_caches`]) replaces it with a fresh
//! random value after the write. Readers then build keys nobody wrote yet, and the orphaned
//! entries expire with their TTL.
//!
//! A read that started before the write and fills the cache after it stores its (stale) page
//! under the **previous** generation, which nobody reads any more: the bump must simply happen
//! after the write is committed, which the [`SmartDatabase`] / [`SmartTransaction`] helpers do.
//!
//! The cache stays an optimisation: when Redis cannot be read the page is loaded from Postgres
//! and not stored, and a failed invalidation is logged (the entries then live until their TTL,
//! which is why it is short, at most [`MAX_READ_CACHE_TTL`]).
//!
//! [`SmartTransaction`]: super::SmartTransaction
//! [`ApiRequestDto::invalidated_read_caches`]: crate::database::db_interface::ApiRequestDto::invalidated_read_caches

use std::fmt::Write as _;

use serde::de::DeserializeOwned;
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::SmartDatabase;

/// Upper bound of a [`ReadCache`] time to live, in seconds.
///
/// The entries of a namespace are only as fresh as their TTL when an invalidation fails, or when
/// another API changes data the page depends on (roles, groups).
pub const MAX_READ_CACHE_TTL: u64 = 300;

/// Time to live of a namespace's generation key, in seconds (7 days). It only has to outlive the
/// entries written under it; refreshed on every invalidation.
pub const READ_CACHE_GENERATION_TTL: u64 = 7 * 24 * 3600;

/// Generation used while a namespace was never invalidated (or its generation key expired).
const INITIAL_GENERATION: &str = "0";

/// A family of cached reads, invalidated together (see the [module documentation](self)).
///
/// ```ignore
/// const CALENDAR_CACHE: ReadCache = ReadCache::new("calendar", 30);
///
/// let key = CALENDAR_CACHE.for_user(auth_user.id, &(params.start, params.end));
/// let events = state
///     .get_smart_db()
///     .cached(&key, async || db.fetch_all::<Event, _>(&view).await)
///     .await?;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadCache {
    namespace: &'static str,
    ttl: u64,
}

impl ReadCache {
    /// A namespace whose entries live `ttl_seconds`, clamped between 1 and [`MAX_READ_CACHE_TTL`].
    #[must_use]
    pub const fn new(namespace: &'static str, ttl_seconds: u64) -> Self {
        let ttl = if ttl_seconds == 0 {
            1
        } else if ttl_seconds > MAX_READ_CACHE_TTL {
            MAX_READ_CACHE_TTL
        } else {
            ttl_seconds
        };
        Self { namespace, ttl }
    }

    /// The namespace name, as written in [`ApiRequestDto::invalidated_read_caches`](crate::database::db_interface::ApiRequestDto::invalidated_read_caches).
    #[must_use]
    pub const fn namespace(&self) -> &'static str {
        self.namespace
    }

    /// Time to live of the entries, in seconds.
    #[must_use]
    pub const fn ttl(&self) -> u64 {
        self.ttl
    }

    /// The entry of `user_id` for `params` (any serializable value: a tuple, the query struct…).
    /// Use it whenever the page depends on who asks (visibility, membership).
    #[must_use]
    pub fn for_user<P: Serialize + ?Sized>(&self, user_id: u64, params: &P) -> ReadCacheKey {
        self.key(format!("u{user_id}"), params)
    }

    /// The entry for `params`, shared by every caller. Only for pages that do **not** depend on
    /// the caller (the same rows whoever asks), such as the `Core_API` directory.
    #[must_use]
    pub fn shared<P: Serialize + ?Sized>(&self, params: &P) -> ReadCacheKey {
        self.key("shared".to_string(), params)
    }

    fn key<P: Serialize + ?Sized>(&self, scope: String, params: &P) -> ReadCacheKey {
        ReadCacheKey {
            cache: *self,
            scope,
            params_hash: hash_params(params),
        }
    }
}

/// One entry of a [`ReadCache`]: the namespace, the caller scope and the parameters' hash. The
/// generation is resolved when the entry is read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadCacheKey {
    cache: ReadCache,
    scope: String,
    params_hash: String,
}

impl ReadCacheKey {
    /// The [`ReadCache`] this entry belongs to.
    #[must_use]
    pub const fn cache(&self) -> ReadCache {
        self.cache
    }

    /// The Redis key of this entry under `generation` (before the role prefix of
    /// [`Redis`](crate::redis::redis_interface::Redis)).
    #[must_use]
    pub fn entry_key(&self, generation: &str) -> String {
        format!(
            "read-cache:{}:{generation}:{}:{}",
            self.cache.namespace, self.scope, self.params_hash
        )
    }
}

/// The Redis key holding the current generation of `namespace`.
#[must_use]
pub fn generation_key(namespace: &str) -> String {
    format!("read-cache:{namespace}:generation")
}

/// SHA-256 of the JSON of `params`, hex encoded: bounded keys, and no request value (search
/// text, e-mail) written in clear in Redis.
fn hash_params<P: Serialize + ?Sized>(params: &P) -> String {
    // A value that cannot be serialized gets a key of its own that is never shared.
    let json =
        serde_json::to_vec(params).unwrap_or_else(|_| uuid::Uuid::new_v4().into_bytes().to_vec());
    let digest = Sha256::digest(json);
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

impl SmartDatabase {
    /// Reads `key` from the cache, or runs `load` and stores its result for the cache's TTL.
    ///
    /// Redis failures never fail the read: when the generation or the entry cannot be read the
    /// page comes from `load` (and is not stored when the generation is unknown); an entry that
    /// does not deserialize as `T` is reloaded.
    ///
    /// # Errors
    ///
    /// Returns the error of `load`; nothing is cached then.
    pub async fn cached<T, E, F>(&self, key: &ReadCacheKey, load: F) -> Result<T, E>
    where
        T: Serialize + DeserializeOwned,
        F: AsyncFnOnce() -> Result<T, E>,
    {
        let redis = self.get_redis();
        let namespace = key.cache.namespace;
        let generation = match redis.secure_get::<String>(&generation_key(namespace)).await {
            Ok(generation) => generation.unwrap_or_else(|| INITIAL_GENERATION.to_string()),
            Err(error) => {
                tracing::debug!(namespace, error = %error, "read cache bypassed: generation unreadable");
                return load().await;
            }
        };
        let entry_key = key.entry_key(&generation);

        if let Ok(Some(json)) = redis.secure_get::<String>(&entry_key).await {
            if let Ok(value) = serde_json::from_str::<T>(&json) {
                return Ok(value);
            }
        }

        let value = load().await?;
        if let Ok(json) = serde_json::to_string(&value) {
            let _ = redis.secure_set_ex(&entry_key, json, key.cache.ttl).await;
        }
        Ok(value)
    }

    /// Invalidates every entry of `namespace` (all users, all parameters) by giving it a new
    /// generation. Call it **after** the write is committed; the views listing the namespace in
    /// [`ApiRequestDto::invalidated_read_caches`](crate::database::db_interface::ApiRequestDto::invalidated_read_caches)
    /// get it for free.
    ///
    /// A Redis failure is logged and otherwise ignored: the entries then live until their TTL.
    pub async fn invalidate_read_cache(&self, namespace: &str) {
        let generation = uuid::Uuid::new_v4().simple().to_string();
        if let Err(error) = self
            .get_redis()
            .set_ex(
                &generation_key(namespace),
                generation,
                READ_CACHE_GENERATION_TTL,
            )
            .await
        {
            tracing::warn!(namespace, error = %error, "read cache invalidation failed, entries expire with their TTL");
        }
    }

    /// [`Self::invalidate_read_cache`] for each namespace.
    pub async fn invalidate_read_caches(&self, namespaces: &[&str]) {
        for namespace in namespaces {
            self.invalidate_read_cache(namespace).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ttl_is_clamped() {
        assert_eq!(ReadCache::new("a", 0).ttl(), 1);
        assert_eq!(ReadCache::new("a", 30).ttl(), 30);
        assert_eq!(ReadCache::new("a", 10_000).ttl(), MAX_READ_CACHE_TTL);
    }

    #[test]
    fn keys_depend_on_user_params_and_generation() {
        let cache = ReadCache::new("calendar", 30);
        let alice = cache.for_user(1, &("2026-10-01", "2026-10-31"));
        let bob = cache.for_user(2, &("2026-10-01", "2026-10-31"));
        let other_period = cache.for_user(1, &("2026-11-01", "2026-11-30"));
        assert_ne!(alice.entry_key("0"), bob.entry_key("0"));
        assert_ne!(alice.entry_key("0"), other_period.entry_key("0"));
        assert_ne!(alice.entry_key("0"), alice.entry_key("1"));
        assert_eq!(alice, cache.for_user(1, &("2026-10-01", "2026-10-31")));
        assert_ne!(
            alice.entry_key("0"),
            cache.shared(&("2026-10-01", "2026-10-31")).entry_key("0")
        );
        assert!(alice
            .entry_key("g")
            .starts_with("read-cache:calendar:g:u1:"));
    }

    #[test]
    fn request_values_are_not_written_in_clear() {
        let key = ReadCache::new("directory", 30).shared(&"alice@example.com");
        assert!(!key.entry_key("0").contains("alice"));
    }
}
