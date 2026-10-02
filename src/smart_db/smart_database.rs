use redis::{FromRedisValue, ToSingleRedisArg};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::database::db_interface::{ApiRequestDto, DbTransaction};
use crate::error::ApiLibError;
use crate::{database::db_interface::Database, redis::redis_interface::Redis};

#[derive(Clone)]
pub struct SmartDatabase {
    db: Database,
    redis: Redis,
}

impl SmartDatabase {
    /// Composes the Postgres and Redis clients into the cache-aside layer.
    #[must_use]
    pub const fn new(db: Database, redis: Redis) -> Self {
        Self { db, redis }
    }

    /// Renvoie le client Redis utilisé par le cache-aside. `Redis` encapsule un
    /// `Arc`, le clone renvoyé partage donc le même pool et le même état de
    /// connexion que celui interrogé par `fetch_*`/`execute`.
    #[must_use]
    pub fn get_redis(&self) -> Redis {
        self.redis.clone()
    }

    /// Writes `value` under `key` unless another request already did, with the view's time to
    /// live in the same command (`SET … [EX ttl] NX`), so a key never lingers without its TTL.
    /// Failures are ignored: the cache is only an optimisation.
    async fn populate_cache<V>(&self, key: &str, value: V, ttl: Option<u64>)
    where
        V: ToSingleRedisArg + Send + Sync,
    {
        let _ = match ttl.filter(|ttl| *ttl > 0) {
            Some(ttl) => self.redis.secure_set_ex(key, value, ttl).await.map(|_| ()),
            None => self.redis.secure_set(key, value).await,
        };
    }

    /// Starts a transaction (see [`SmartTransaction`]).
    ///
    /// # Errors
    ///
    /// Returns an [`ApiLibError`] if no connection can be obtained or `BEGIN` fails.
    pub async fn begin(&self) -> Result<SmartTransaction, ApiLibError> {
        Ok(SmartTransaction {
            tx: self.db.begin().await?,
            redis: self.redis.clone(),
            pending_invalidations: Vec::new(),
        })
    }

    /// Runs `work` in a transaction: committed when it returns `Ok`, rolled back when it returns
    /// `Err` (MAIR-420). Prefer it to [`Self::begin`], which leaves the `commit()` to the caller.
    ///
    /// The closure gets the [`SmartTransaction`] to run its queries on; its error type only has
    /// to accept an [`ApiLibError`], so an API can use its own endpoint error enum.
    ///
    /// ```ignore
    /// let chat_id = state
    ///     .get_smart_db()
    ///     .transaction(async |tx| {
    ///         let chat: ChatId = tx.fetch_one(&CreateChatView::new(&name)).await?;
    ///         for member in &members {
    ///             tx.execute(&AddChatMemberView::new(chat.id, *member)).await?;
    ///         }
    ///         Ok::<_, ApiLibError>(chat.id)
    ///     })
    ///     .await?;
    /// ```
    ///
    /// # Errors
    ///
    /// Returns the error of `work` (after the rollback), or an [`ApiLibError`] converted into `E`
    /// when `BEGIN` or `COMMIT` fails; in every error case nothing is applied.
    pub async fn transaction<R, E, F>(&self, work: F) -> Result<R, E>
    where
        F: AsyncFnOnce(&mut SmartTransaction) -> Result<R, E>,
        E: From<ApiLibError>,
    {
        let mut tx = self.begin().await?;
        match work(&mut tx).await {
            Ok(value) => {
                tx.commit().await?;
                Ok(value)
            }
            Err(error) => {
                // A failed ROLLBACK leaves nothing applied either: Postgres aborts the
                // transaction when the connection goes back to the pool.
                let _ = tx.rollback().await;
                Err(error)
            }
        }
    }

    /// # Errors
    ///
    /// Renvoie une [`ApiLibError`] si la requête en base échoue. Les erreurs Redis sont
    /// ignorées : le cache n'est qu'une optimisation.
    pub async fn execute<Q>(&self, query: Q) -> Result<(), ApiLibError>
    where
        Q: ApiRequestDto,
    {
        // 1. On exécute la modification en base (INSERT, UPDATE ou DELETE) -> Erreur critique remontée[cite: 1]
        self.db.execute(&query).await?;

        // 2. Si la vue déclare une clé de cache, on l'invalide (échec Redis ignoré pour l'utilisateur)[cite: 1]
        if let Some(ref key) = query.cache_key() {
            let _ = self.redis.secure_delete(key).await;
        }

        Ok(())
    }

    /// # Errors
    ///
    /// Renvoie une [`ApiLibError`] si la requête en base échoue. Les erreurs Redis sont
    /// ignorées : le cache n'est qu'une optimisation.
    pub async fn fetch_one<T, Q>(&self, query: &Q) -> Result<T, ApiLibError>
    where
        T: DeserializeOwned + Serialize,
        Q: ApiRequestDto,
    {
        let cache_key = query.cache_key();
        let cache_ttl = query.cache_ttl();

        // 1. Si la vue a une clé de cache, on interroge Redis (silencieux en cas d'échec)[cite: 1]
        if let Some(ref key) = cache_key {
            if let Ok(Some(json_str)) = self.redis.secure_get::<String>(key).await {
                if let Ok(value) = serde_json::from_str::<T>(&json_str) {
                    return Ok(value); // Cache Hit ![cite: 1]
                }
            }
        }

        // 2. Sinon, on tape dans PostgreSQL -> Les erreurs de DB remontent à l'API[cite: 1]
        let value: T = self.db.fetch_one(query).await?;

        // 3. On remplit le cache et on applique le TTL si défini (non-bloquant)[cite: 1]
        if let Some(ref key) = cache_key {
            if let Ok(json_str) = serde_json::to_string(&value) {
                self.populate_cache(key, json_str, cache_ttl).await;
            }
        }

        Ok(value)
    }

    /// # Errors
    ///
    /// Renvoie une [`ApiLibError`] si la requête en base échoue. Les erreurs Redis sont
    /// ignorées : le cache n'est qu'une optimisation.
    pub async fn fetch_all<T, Q>(&self, query: &Q) -> Result<Vec<T>, ApiLibError>
    where
        T: DeserializeOwned + Serialize,
        Q: ApiRequestDto,
    {
        let cache_key = query.cache_key();
        let cache_ttl = query.cache_ttl();

        // 1. Si la vue déclare une clé de cache, on tente de récupérer la liste (silencieux si Redis échoue)[cite: 1]
        if let Some(ref key) = cache_key {
            if let Ok(Some(json_str)) = self.redis.secure_get::<String>(key).await {
                if let Ok(values) = serde_json::from_str::<Vec<T>>(&json_str) {
                    return Ok(values); // Cache Hit ![cite: 1]
                }
            }
        }

        // 2. Cache Miss : on interroge PostgreSQL -> Erreur remontée[cite: 1]
        let values: Vec<T> = self.db.fetch_all(query).await?;

        // 3. Stockage dans Redis + application du TTL si défini (non-bloquant)[cite: 1]
        if let Some(ref key) = cache_key {
            if let Ok(json_str) = serde_json::to_string(&values) {
                self.populate_cache(key, json_str, cache_ttl).await;
            }
        }

        Ok(values)
    }

    /// # Errors
    ///
    /// Renvoie une [`ApiLibError`] si la requête en base échoue. Les erreurs Redis sont
    /// ignorées : le cache n'est qu'une optimisation.
    pub async fn fetch_scalar<T, Q>(&self, query: &Q) -> Result<T, ApiLibError>
    where
        // Contraintes SQL existantes[cite: 1]
        T: for<'r> sqlx::Decode<'r, sqlx::Postgres> + sqlx::Type<sqlx::Postgres> + Send + Unpin,
        // Contraintes Redis pour lire et écrire le scalaire directement[cite: 1]
        T: FromRedisValue + ToSingleRedisArg + Send + Sync + std::marker::Copy,
        Q: ApiRequestDto,
    {
        let cache_key = query.cache_key();
        let cache_ttl = query.cache_ttl();

        // 1. Si la vue déclare une clé de cache, on tente de récupérer le scalaire (silencieux)[cite: 1]
        if let Some(ref key) = cache_key {
            if let Ok(Some(value)) = self.redis.secure_get::<T>(key).await {
                return Ok(value); // Cache Hit ![cite: 1]
            }
        }

        // 2. Cache Miss : on interroge PostgreSQL -> Erreur remontée[cite: 1]
        let value: T = self.db.fetch_scalar(query).await?;

        // 3. Stockage dans Redis + application du TTL si défini (non-bloquant)[cite: 1]
        if let Some(ref key) = cache_key {
            self.populate_cache(key, value, cache_ttl).await;
        }

        Ok(value)
    }
}

/// A transaction of the [`SmartDatabase`], for the API operations made of several queries that
/// must be applied all together or not at all.
///
/// - Every query runs on the transaction's connection. Reads **bypass the cache**: they must
///   see the transaction's own uncommitted writes, and must not put them in Redis.
/// - The cache keys of the views passed to [`Self::execute`] are invalidated only after a
///   successful [`Self::commit`].
/// - Dropping it without committing rolls the transaction back, so an early `?` return undoes
///   the queries already run.
///
/// ```ignore
/// let mut tx = state.get_smart_db().begin().await?;
/// tx.execute(&CreateProjectView::new(...)).await?;
/// tx.execute(&AddProjectMemberView::new(...)).await?;
/// tx.commit().await?;
/// ```
pub struct SmartTransaction {
    tx: DbTransaction,
    redis: Redis,
    pending_invalidations: Vec<String>,
}

impl SmartTransaction {
    /// # Errors
    ///
    /// Returns an [`ApiLibError`] if the query fails; the transaction is then aborted by
    /// Postgres and must be dropped or rolled back.
    pub async fn execute<Q>(&mut self, query: &Q) -> Result<(), ApiLibError>
    where
        Q: ApiRequestDto,
    {
        self.tx.execute(query).await?;
        if let Some(key) = query.cache_key() {
            self.pending_invalidations.push(key);
        }
        Ok(())
    }

    /// # Errors
    ///
    /// Same as [`SmartDatabase::fetch_one`], without the cache.
    pub async fn fetch_one<T, Q>(&mut self, query: &Q) -> Result<T, ApiLibError>
    where
        T: DeserializeOwned,
        Q: ApiRequestDto,
    {
        Ok(self.tx.fetch_one(query).await?)
    }

    /// # Errors
    ///
    /// Same as [`SmartDatabase::fetch_all`], without the cache.
    pub async fn fetch_all<T, Q>(&mut self, query: &Q) -> Result<Vec<T>, ApiLibError>
    where
        T: DeserializeOwned,
        Q: ApiRequestDto,
    {
        Ok(self.tx.fetch_all(query).await?)
    }

    /// # Errors
    ///
    /// Same as [`SmartDatabase::fetch_scalar`], without the cache.
    pub async fn fetch_scalar<T, Q>(&mut self, query: &Q) -> Result<T, ApiLibError>
    where
        T: for<'r> sqlx::Decode<'r, sqlx::Postgres> + sqlx::Type<sqlx::Postgres> + Send + Unpin,
        Q: ApiRequestDto,
    {
        Ok(self.tx.fetch_scalar(query).await?)
    }

    /// Applies the transaction, then invalidates the cache keys of its writes (Redis failures
    /// ignored, like [`SmartDatabase::execute`]).
    ///
    /// # Errors
    ///
    /// Returns an [`ApiLibError`] if `COMMIT` fails; nothing is applied nor invalidated.
    pub async fn commit(self) -> Result<(), ApiLibError> {
        self.tx.commit().await?;
        for key in &self.pending_invalidations {
            let _ = self.redis.delete(key).await;
        }
        Ok(())
    }

    /// Discards the transaction.
    ///
    /// # Errors
    ///
    /// Returns an [`ApiLibError`] if `ROLLBACK` fails.
    pub async fn rollback(self) -> Result<(), ApiLibError> {
        Ok(self.tx.rollback().await?)
    }
}
