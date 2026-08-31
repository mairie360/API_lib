use std::sync::Arc;

use deadpool_redis::{Config, Pool, PoolError, Runtime};
use redis::{AsyncCommands, FromRedisValue, ToSingleRedisArg};
use thiserror::Error;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub enum RedisParam {
    Text(String),
    I64(i64),
    I32(i32),
    Bytes(Vec<u8>),
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RedisError {
    #[error("Erreur de pool Redis : {0}")]
    Pool(String),
    #[error("Erreur du driver Redis : {0}")]
    Driver(String),
    #[error("Erreur interne Redis : {0}")]
    Internal(String),
    #[error("Erreur de valeur Redis : {0}")]
    Value(String),
}

pub trait RedisRequestDto {
    fn key(&self) -> &str;
    fn args(&self) -> &[RedisParam];
}

#[derive(Clone)]
pub struct Redis {
    inner: Arc<RedisInner>,
}

struct RedisInner {
    redis_url: String,
    pool: Mutex<Option<Pool>>,
}

impl Redis {
    pub fn new(redis_url: &str) -> Self {
        let redis_cfg = Config::from_url(redis_url);
        let redis_pool = match redis_cfg.create_pool(Some(Runtime::Tokio1)) {
            Ok(pool) => Some(pool),
            Err(e) => {
                eprintln!("Failed to connect to Redis: {}", e);
                None
            }
        };
        Self {
            inner: Arc::new(RedisInner {
                redis_url: redis_url.to_string(),
                pool: Mutex::new(redis_pool),
            }),
        }
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
                eprintln!("Failed to connect to Redis: {}", e);
                return Err(PoolError::Closed);
            }
        };

        *guard = Some(pool.clone());
        Ok(pool)
    }

    pub async fn get<T, Q>(&self, query: Q) -> Result<T, RedisError>
    where
        Q: RedisRequestDto,
        T: FromRedisValue, // <--- C'est ici que la magie opère
    {
        let pool = self
            .get_pool()
            .await
            .map_err(|e| RedisError::Pool(e.to_string()))?;

        let mut conn = pool
            .get()
            .await
            .map_err(|e| RedisError::Pool(e.to_string()))?;

        let result: T = conn
            .get(query.key())
            .await
            .map_err(|e| RedisError::Driver(e.to_string()))?;

        Ok(result)
    }

    pub async fn set<V>(&self, key: &str, value: V) -> Result<(), RedisError>
    where
        V: ToSingleRedisArg + Send + Sync,
    {
        let pool = self
            .get_pool()
            .await
            .map_err(|e| RedisError::Pool(e.to_string()))?;

        let mut conn = pool
            .get()
            .await
            .map_err(|e| RedisError::Pool(e.to_string()))?;

        let _: () = conn
            .set(key, value)
            .await
            .map_err(|e| RedisError::Driver(e.to_string()))?;

        Ok(())
    }

    pub async fn delete(&self, key: &str) -> Result<(), RedisError> {
        let pool = self
            .get_pool()
            .await
            .map_err(|e| RedisError::Pool(e.to_string()))?;

        let mut conn = pool
            .get()
            .await
            .map_err(|e| RedisError::Pool(e.to_string()))?;

        // Ajout explicite du turbofish ::<()> pour éviter l'erreur de typage
        let result = conn
            .del(key)
            .await
            .map_err(|e| RedisError::Driver(e.to_string()))?;

        Ok(result)
    }

    pub async fn key_exist(&self, key: &str) -> Result<bool, RedisError> {
        let pool = self
            .get_pool()
            .await
            .map_err(|e| RedisError::Pool(e.to_string()))?;

        let mut conn = pool
            .get()
            .await
            .map_err(|e| RedisError::Pool(e.to_string()))?;

        // Ajout explicite du turbofish ::<()> pour éviter l'erreur de typage
        let result: bool = conn
            .exists(key)
            .await
            .map_err(|e| RedisError::Driver(e.to_string()))?;

        Ok(result)
    }

    pub async fn secure_get<T, Q>(&self, query: Q) -> Result<Option<T>, RedisError>
    where
        T: FromRedisValue,
        Q: RedisRequestDto,
    {
        let pool = self
            .get_pool()
            .await
            .map_err(|e| RedisError::Pool(e.to_string()))?;

        let mut conn = pool
            .get()
            .await
            .map_err(|e| RedisError::Pool(e.to_string()))?;

        if !conn.exists(query.key()).await.unwrap_or(false) {
            return Ok(None);
        }

        // Ajout explicite du turbofish ::<()> pour éviter l'erreur de typage
        let result = self
            .get::<T, Q>(query)
            .await
            .map_err(|e| RedisError::Driver(e.to_string()))?;

        Ok(Some(result))
    }

    pub async fn secure_set<V>(&self, key: &str, value: V) -> Result<(), RedisError>
    where
        V: ToSingleRedisArg + Send + Sync,
    {
        let pool = self
            .get_pool()
            .await
            .map_err(|e| RedisError::Pool(e.to_string()))?;

        let mut conn = pool
            .get()
            .await
            .map_err(|e| RedisError::Pool(e.to_string()))?;

        if conn.exists(key).await.unwrap_or(false) {
            return Ok(());
        }

        self.set(key, value)
            .await
            .map_err(|e| RedisError::Driver(e.to_string()))?;

        Ok(())
    }

    pub async fn secure_delete(&self, key: &str) -> Result<(), RedisError> {
        let pool = self
            .get_pool()
            .await
            .map_err(|e| RedisError::Pool(e.to_string()))?;

        let mut conn = pool
            .get()
            .await
            .map_err(|e| RedisError::Pool(e.to_string()))?;

        if !conn.exists(key).await.unwrap_or(false) {
            return Ok(());
        }

        self.delete(key)
            .await
            .map_err(|e| RedisError::Driver(e.to_string()))?;

        Ok(())
    }
}
