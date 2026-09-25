use std::sync::Arc;

use crate::redis::error::RedisError;
use deadpool_redis::{Config, Connection, Pool, PoolError, Runtime};
use redis::{AsyncCommands, FromRedisValue, ToSingleRedisArg};
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub enum RedisParam {
    Text(String),
    I64(i64),
    I32(i32),
    Bytes(Vec<u8>),
}

/// Redis client of an API (cache-aside of [`SmartDatabase`](crate::smart_db::SmartDatabase),
/// one-time tokens...).
///
/// # Key prefix (MAIR-267)
///
/// Every key passed to the public methods is prefixed with `<role>:`, because the Redis ACL of
/// the platform only lets each API role touch `~<role>:*` (Deploiment chart). The role is, in
/// order (see [`resolve_key_prefix`]):
/// 1. `REDIS_KEY_PREFIX`, when set and not empty (explicit override);
/// 2. the username of `REDIS_URL` (`redis://core-api:<password>@redis:6379`);
/// 3. `REDIS_USERNAME`.
///
/// Without any of them (local Redis without ACL) keys are sent **unprefixed**, as before: a dev
/// Redis without users has no role to scope keys to, and keeping the historical keys avoids
/// breaking local stacks. Callers never add the prefix themselves.
///
/// The only unprefixed keys are the shared JWT revocation list (`revoked:<sid>`), reachable only
/// through [`crate::jwt_manager::revoke_session`] / [`crate::jwt_manager::is_session_revoked`].
///
/// Commands used: `GET`, `SET` (with `EX` / `NX` options), `DEL`, `EXISTS`, `EXPIRE`, the ones
/// the chart's `aclCommands` grant. `SETEX` is **not** granted and must not be used.
#[derive(Clone)]
pub struct Redis {
    inner: Arc<RedisInner>,
}

struct RedisInner {
    redis_url: String,
    key_prefix: Option<String>,
    pool: Mutex<Option<Pool>>,
}

/// Username in a `redis://user:password@host:port` URL, if any.
fn username_from_url(redis_url: &str) -> Option<String> {
    let (_, rest) = redis_url.split_once("://")?;
    let (userinfo, _) = rest.rsplit_once('@')?;
    let username = userinfo.split(':').next().unwrap_or_default();
    (!username.is_empty()).then(|| username.to_string())
}

/// Key prefix (role) for `redis_url`, following the order documented on [`Redis`].
#[must_use]
pub fn resolve_key_prefix(redis_url: &str) -> Option<String> {
    let non_empty = |value: String| (!value.is_empty()).then_some(value);
    std::env::var("REDIS_KEY_PREFIX")
        .ok()
        .and_then(non_empty)
        .or_else(|| username_from_url(redis_url))
        .or_else(|| std::env::var("REDIS_USERNAME").ok().and_then(non_empty))
}

impl Redis {
    /// Client for `redis_url`, with the key prefix resolved from the environment
    /// ([`resolve_key_prefix`]).
    #[must_use]
    pub fn new(redis_url: &str) -> Self {
        Self::with_key_prefix(redis_url, resolve_key_prefix(redis_url).as_deref())
    }

    /// Client for `redis_url` with an explicit key prefix (`None` = keys sent verbatim).
    #[must_use]
    pub fn with_key_prefix(redis_url: &str, key_prefix: Option<&str>) -> Self {
        let redis_cfg = Config::from_url(redis_url);
        let redis_pool = match redis_cfg.create_pool(Some(Runtime::Tokio1)) {
            Ok(pool) => Some(pool),
            Err(e) => {
                eprintln!("Failed to connect to Redis: {e}");
                None
            }
        };
        Self {
            inner: Arc::new(RedisInner {
                redis_url: redis_url.to_string(),
                key_prefix: key_prefix
                    .filter(|prefix| !prefix.is_empty())
                    .map(str::to_string),
                pool: Mutex::new(redis_pool),
            }),
        }
    }

    /// The role prefix applied to every key, without the `:` separator.
    #[must_use]
    pub fn key_prefix(&self) -> Option<&str> {
        self.inner.key_prefix.as_deref()
    }

    /// The key actually sent to Redis for `key`: `<prefix>:<key>`, or `key` without prefix.
    #[must_use]
    pub fn full_key(&self, key: &str) -> String {
        self.inner
            .key_prefix
            .as_ref()
            .map_or_else(|| key.to_string(), |prefix| format!("{prefix}:{key}"))
    }

    pub async fn is_connected(&self) -> bool {
        self.inner.pool.lock().await.is_some()
    }

    async fn get_pool(&self) -> Result<Pool, PoolError> {
        let mut guard = self.inner.pool.lock().await;
        if let Some(pool) = &*guard {
            return Ok(pool.clone());
        }

        // Connexion paresseuse si le pool est vide
        let redis_cfg = Config::from_url(&self.inner.redis_url);
        let pool = match redis_cfg.create_pool(Some(Runtime::Tokio1)) {
            Ok(pool) => pool,
            Err(e) => {
                eprintln!("Failed to connect to Redis: {e}");
                return Err(PoolError::Closed);
            }
        };

        *guard = Some(pool.clone());
        drop(guard);
        Ok(pool)
    }

    async fn connection(&self) -> Result<Connection, RedisError> {
        let pool = self
            .get_pool()
            .await
            .map_err(|e| RedisError::Pool(e.to_string()))?;
        pool.get()
            .await
            .map_err(|e| RedisError::Pool(e.to_string()))
    }

    // --- Raw commands on full keys (already prefixed, or the shared revocation keys) ---

    async fn raw_get<T: FromRedisValue>(&self, full_key: &str) -> Result<T, RedisError> {
        let mut conn = self.connection().await?;
        conn.get(full_key)
            .await
            .map_err(|e| RedisError::Driver(e.to_string()))
    }

    async fn raw_set<V>(&self, full_key: &str, value: V) -> Result<(), RedisError>
    where
        V: ToSingleRedisArg + Send + Sync,
    {
        let mut conn = self.connection().await?;
        conn.set::<_, _, ()>(full_key, value)
            .await
            .map_err(|e| RedisError::Driver(e.to_string()))
    }

    /// `SET full_key value EX seconds [NX]`. Returns whether the key was written (always `true`
    /// without `NX`). `SET` with options, not `SETEX`: only `SET` is granted by the ACL.
    async fn raw_set_ex<V>(
        &self,
        full_key: &str,
        value: V,
        seconds: u64,
        only_if_absent: bool,
    ) -> Result<bool, RedisError>
    where
        V: ToSingleRedisArg + Send + Sync,
    {
        if seconds == 0 {
            return Err(RedisError::Value(
                "a time to live must be at least 1 second".to_string(),
            ));
        }
        let mut conn = self.connection().await?;
        let mut cmd = redis::cmd("SET");
        cmd.arg(full_key).arg(value).arg("EX").arg(seconds);
        if only_if_absent {
            cmd.arg("NX");
        }
        // `OK` when written, nil when `NX` found the key.
        let reply: Option<String> = cmd
            .query_async(&mut conn)
            .await
            .map_err(|e| RedisError::Driver(e.to_string()))?;
        Ok(reply.is_some())
    }

    async fn raw_delete(&self, full_key: &str) -> Result<(), RedisError> {
        let mut conn = self.connection().await?;
        conn.del::<_, ()>(full_key)
            .await
            .map_err(|e| RedisError::Driver(e.to_string()))
    }

    async fn raw_expire(&self, full_key: &str, seconds: u64) -> Result<(), RedisError> {
        let mut conn = self.connection().await?;
        conn.expire::<_, ()>(full_key, i64::try_from(seconds).unwrap_or(i64::MAX))
            .await
            .map_err(|e| RedisError::Driver(e.to_string()))
    }

    async fn raw_exists(&self, full_key: &str) -> Result<bool, RedisError> {
        let mut conn = self.connection().await?;
        conn.exists(full_key)
            .await
            .map_err(|e| RedisError::Driver(e.to_string()))
    }

    // --- Shared, unprefixed keys: crate-private, for the revocation list only ---

    /// `SET key value EX seconds` on a key **not** prefixed with the role.
    pub(crate) async fn set_ex_shared<V>(
        &self,
        shared_key: &str,
        value: V,
        seconds: u64,
    ) -> Result<(), RedisError>
    where
        V: ToSingleRedisArg + Send + Sync,
    {
        self.raw_set_ex(shared_key, value, seconds, false)
            .await
            .map(|_| ())
    }

    /// `EXISTS key` on a key **not** prefixed with the role.
    pub(crate) async fn exists_shared(&self, shared_key: &str) -> Result<bool, RedisError> {
        self.raw_exists(shared_key).await
    }

    // --- Public API: every key is prefixed with the role ---

    /// # Errors
    ///
    /// Renvoie `RedisError::Pool` si aucune connexion ne peut être obtenue et
    /// `RedisError::Driver` si Redis rejette la commande.
    pub async fn get<T>(&self, key: &str) -> Result<T, RedisError>
    where
        T: FromRedisValue,
    {
        self.raw_get(&self.full_key(key)).await
    }

    /// Sets `key` without time to live. Prefer [`Self::set_ex`] for anything temporary.
    ///
    /// # Errors
    ///
    /// Renvoie `RedisError::Pool` si aucune connexion ne peut être obtenue et
    /// `RedisError::Driver` si Redis rejette la commande.
    pub async fn set<V>(&self, key: &str, value: V) -> Result<(), RedisError>
    where
        V: ToSingleRedisArg + Send + Sync,
    {
        self.raw_set(&self.full_key(key), value).await
    }

    /// Sets `key` to `value` with a time to live of `seconds`, atomically (`SET key value EX
    /// seconds`), overwriting any previous value.
    ///
    /// # Errors
    ///
    /// Returns `RedisError::Value` when `seconds` is 0, `RedisError::Pool` when no connection can
    /// be obtained and `RedisError::Driver` when Redis rejects the command.
    pub async fn set_ex<V>(&self, key: &str, value: V, seconds: u64) -> Result<(), RedisError>
    where
        V: ToSingleRedisArg + Send + Sync,
    {
        self.raw_set_ex(&self.full_key(key), value, seconds, false)
            .await
            .map(|_| ())
    }

    /// Sets `key` to `value` with a time to live of `seconds` only if `key` does not exist
    /// (`SET key value EX seconds NX`, atomic). Returns whether the key was written.
    ///
    /// # Errors
    ///
    /// Returns `RedisError::Value` when `seconds` is 0, `RedisError::Pool` when no connection can
    /// be obtained and `RedisError::Driver` when Redis rejects the command.
    pub async fn secure_set_ex<V>(
        &self,
        key: &str,
        value: V,
        seconds: u64,
    ) -> Result<bool, RedisError>
    where
        V: ToSingleRedisArg + Send + Sync,
    {
        self.raw_set_ex(&self.full_key(key), value, seconds, true)
            .await
    }

    /// # Errors
    ///
    /// Renvoie `RedisError::Pool` si aucune connexion ne peut être obtenue et
    /// `RedisError::Driver` si Redis rejette la commande.
    pub async fn delete(&self, key: &str) -> Result<(), RedisError> {
        self.raw_delete(&self.full_key(key)).await
    }

    /// # Errors
    ///
    /// Renvoie `RedisError::Pool` si aucune connexion ne peut être obtenue et
    /// `RedisError::Driver` si Redis rejette la commande.
    pub async fn expire(&self, key: &str, seconds: u64) -> Result<(), RedisError> {
        self.raw_expire(&self.full_key(key), seconds).await
    }

    /// # Errors
    ///
    /// Renvoie `RedisError::Pool` si aucune connexion ne peut être obtenue et
    /// `RedisError::Driver` si Redis rejette la commande.
    pub async fn key_exist(&self, key: &str) -> Result<bool, RedisError> {
        self.raw_exists(&self.full_key(key)).await
    }

    /// # Errors
    ///
    /// Renvoie `RedisError::Pool` si aucune connexion ne peut être obtenue et
    /// `RedisError::Driver` si Redis rejette la commande.
    pub async fn secure_get<T>(&self, key: &str) -> Result<Option<T>, RedisError>
    where
        T: FromRedisValue,
    {
        let full_key = self.full_key(key);
        if !self.raw_exists(&full_key).await.unwrap_or(false) {
            return Ok(None);
        }
        self.raw_get::<T>(&full_key)
            .await
            .map(Some)
            .map_err(|e| RedisError::Driver(e.to_string()))
    }

    /// # Errors
    ///
    /// Renvoie `RedisError::Pool` si aucune connexion ne peut être obtenue et
    /// `RedisError::Driver` si Redis rejette la commande.
    pub async fn secure_set<V>(&self, key: &str, value: V) -> Result<(), RedisError>
    where
        V: ToSingleRedisArg + Send + Sync,
    {
        let full_key = self.full_key(key);
        if self.raw_exists(&full_key).await.unwrap_or(false) {
            return Ok(());
        }
        self.raw_set(&full_key, value)
            .await
            .map_err(|e| RedisError::Driver(e.to_string()))
    }

    /// # Errors
    ///
    /// Renvoie `RedisError::Pool` si aucune connexion ne peut être obtenue et
    /// `RedisError::Driver` si Redis rejette la commande.
    pub async fn secure_delete(&self, key: &str) -> Result<(), RedisError> {
        let full_key = self.full_key(key);
        if !self.raw_exists(&full_key).await.unwrap_or(false) {
            return Ok(());
        }
        self.raw_delete(&full_key)
            .await
            .map_err(|e| RedisError::Driver(e.to_string()))
    }

    /// # Errors
    ///
    /// Renvoie `RedisError::Pool` si aucune connexion ne peut être obtenue et
    /// `RedisError::Driver` si Redis rejette la commande.
    pub async fn secure_expire(&self, key: &str, seconds: u64) -> Result<(), RedisError> {
        let full_key = self.full_key(key);
        if !self.raw_exists(&full_key).await.unwrap_or(false) {
            return Ok(());
        }
        self.raw_expire(&full_key, seconds)
            .await
            .map_err(|e| RedisError::Driver(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::username_from_url;

    #[test]
    fn username_is_read_from_the_url() {
        assert_eq!(
            username_from_url("redis://core-api:s3cr%40t@redis:6379"),
            Some("core-api".to_string())
        );
        assert_eq!(
            username_from_url("redis://core-api@redis:6379/0"),
            Some("core-api".to_string())
        );
        assert_eq!(username_from_url("redis://:password@redis:6379"), None);
        assert_eq!(username_from_url("redis://redis:6379"), None);
        assert_eq!(username_from_url(""), None);
    }
}
