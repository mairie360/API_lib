use redis::{FromRedisValue, ToSingleRedisArg};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::database::db_interface::ApiRequestDto;
use crate::{database::db_interface::Database, redis::redis_interface::Redis};

#[derive(Clone)]
pub struct SmartDatabase {
    db: Database,
    redis: Redis,
}

impl SmartDatabase {
    /// Constructeur pour initialiser le SmartDatabase avec tes deux interfaces
    pub fn new(db: Database, redis: Redis) -> Self {
        Self { db, redis }
    }

    pub async fn execute<Q>(&self, query: Q) -> Result<(), Box<dyn std::error::Error>>
    where
        Q: ApiRequestDto,
    {
        // 1. On exécute la modification en base (INSERT, UPDATE ou DELETE)
        self.db.execute(&query).await?;

        // 2. Si la vue déclare une clé de cache, on l'invalide systématiquement
        if let Some(ref key) = query.cache_key() {
            let _ = self.redis.secure_delete(key).await;
        }

        Ok(())
    }

    pub async fn fetch_one<T, Q>(&self, query: &Q) -> Result<T, Box<dyn std::error::Error>>
    where
        T: DeserializeOwned + Serialize,
        Q: ApiRequestDto,
    {
        let cache_key = query.cache_key();
        let cache_ttl = query.cache_ttl();

        // 1. Si la vue a une clé de cache, on interroge directement Redis
        if let Some(ref key) = cache_key {
            if let Ok(Some(json_str)) = self.redis.secure_get::<String>(key).await {
                if let Ok(value) = serde_json::from_str::<T>(&json_str) {
                    return Ok(value); // Cache Hit !
                }
            }
        }

        // 2. Sinon, on tape dans PostgreSQL
        let value: T = self.db.fetch_one(query).await?;

        // 3. On remplit le cache et on applique le TTL si défini
        if let Some(ref key) = cache_key {
            if let Ok(json_str) = serde_json::to_string(&value) {
                let _ = self.redis.secure_set(key, json_str).await;

                if let Some(ttl) = cache_ttl {
                    let _ = self.redis.secure_expire(key, ttl).await;
                }
            }
        }

        Ok(value)
    }

    pub async fn fetch_all<T, Q>(&self, query: &Q) -> Result<Vec<T>, Box<dyn std::error::Error>>
    where
        T: DeserializeOwned + Serialize,
        Q: ApiRequestDto,
    {
        let cache_key = query.cache_key();
        let cache_ttl = query.cache_ttl();

        // 1. Si la vue déclare une clé de cache, on tente de récupérer la liste complète
        if let Some(ref key) = cache_key {
            if let Ok(Some(json_str)) = self.redis.secure_get::<String>(key).await {
                if let Ok(values) = serde_json::from_str::<Vec<T>>(&json_str) {
                    return Ok(values); // Cache Hit !
                }
            }
        }

        // 2. Cache Miss : on interroge PostgreSQL
        let values: Vec<T> = self.db.fetch_all(query).await?;

        // 3. Stockage dans Redis + application du TTL si défini
        if let Some(ref key) = cache_key {
            if let Ok(json_str) = serde_json::to_string(&values) {
                let _ = self.redis.secure_set(key, json_str).await;

                if let Some(ttl) = cache_ttl {
                    let _ = self.redis.secure_expire(key, ttl).await;
                }
            }
        }

        Ok(values)
    }

    pub async fn fetch_scalar<T, Q>(&self, query: &Q) -> Result<T, Box<dyn std::error::Error>>
    where
        // Contraintes SQL existantes
        T: for<'r> sqlx::Decode<'r, sqlx::Postgres> + sqlx::Type<sqlx::Postgres> + Send + Unpin,
        // Contraintes Redis pour lire et écrire le scalaire directement
        T: FromRedisValue + ToSingleRedisArg + Send + Sync + std::marker::Copy,
        Q: ApiRequestDto,
    {
        let cache_key = query.cache_key();
        let cache_ttl = query.cache_ttl();

        // 1. Si la vue déclare une clé de cache, on tente de récupérer le scalaire
        if let Some(ref key) = cache_key {
            if let Ok(Some(value)) = self.redis.secure_get::<T>(key).await {
                return Ok(value); // Cache Hit !
            }
        }

        // 2. Cache Miss : on interroge PostgreSQL
        let value: T = self.db.fetch_scalar(query).await?;

        // 3. Stockage dans Redis + application du TTL si défini
        if let Some(ref key) = cache_key {
            let _ = self.redis.secure_set(key, value).await;

            if let Some(ttl) = cache_ttl {
                let _ = self.redis.secure_expire(key, ttl).await;
            }
        }

        Ok(value)
    }
}
