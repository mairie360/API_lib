use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use sqlx::postgres::PgArguments;
use sqlx::{Arguments, PgPool};
use std::net::IpAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::database::error::DbError;

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum QueryParam {
    I32(i32),
    I64(i64),
    Bool(bool),
    Text(String),
    Uuid(Uuid),
    DateTime(DateTime<Utc>),
    IpAddr(IpAddr),
    OptionI32(Option<i32>),
}

impl QueryParam {
    /// # Panics
    ///
    /// Panique si le paramètre n'est pas de la variante `QueryParam::Text`.
    #[must_use]
    pub fn as_text(&self) -> &str {
        match self {
            Self::Text(s) => s,
            _ => panic!("Expected Text, got {self:?}"),
        }
    }

    /// # Panics
    ///
    /// Panique si le paramètre n'est pas de la variante `QueryParam::I32`.
    #[must_use]
    pub fn as_i32(&self) -> i32 {
        match self {
            Self::I32(v) => *v,
            _ => panic!("Expected I32, got {self:?}"),
        }
    }

    /// # Panics
    ///
    /// Panique si le paramètre n'est pas de la variante `QueryParam::I64`.
    #[must_use]
    pub fn as_i64(&self) -> i64 {
        match self {
            Self::I64(v) => *v,
            _ => panic!("Expected I64, got {self:?}"),
        }
    }

    /// # Panics
    ///
    /// Panique si le paramètre n'est pas de la variante `QueryParam::Bool`.
    #[must_use]
    pub fn as_bool(&self) -> bool {
        match self {
            Self::Bool(v) => *v,
            _ => panic!("Expected Bool, got {self:?}"),
        }
    }

    /// # Panics
    ///
    /// Panique si le paramètre n'est pas de la variante `QueryParam::Uuid`.
    #[must_use]
    pub fn as_uuid(&self) -> Uuid {
        match self {
            Self::Uuid(v) => *v,
            _ => panic!("Expected Uuid, got {self:?}"),
        }
    }

    /// # Panics
    ///
    /// Panique si le paramètre n'est pas de la variante `QueryParam::DateTime`.
    #[must_use]
    pub fn as_datetime(&self) -> DateTime<Utc> {
        match self {
            Self::DateTime(v) => *v,
            _ => panic!("Expected DateTime, got {self:?}"),
        }
    }

    /// # Panics
    ///
    /// Panique si le paramètre n'est pas de la variante `QueryParam::IpAddr`.
    #[must_use]
    pub fn as_ipaddr(&self) -> IpAddr {
        match self {
            Self::IpAddr(v) => *v,
            _ => panic!("Expected IpAddr, got {self:?}"),
        }
    }

    /// # Panics
    ///
    /// Panique si le paramètre n'est pas de la variante `QueryParam::OptionI32`.
    #[must_use]
    pub fn as_option_i32(&self) -> Option<i32> {
        match self {
            Self::OptionI32(v) => *v,
            _ => panic!("Expected OptionI32, got {self:?}"),
        }
    }
}

/// Converts an API id (`u64`) to an `INT4` SQL id.
///
/// Values above `i32::MAX` saturate instead of wrapping: `as i32` would turn `2^32 + 1` into
/// `1`, the id of another row. For ids coming from a request, prefer [`super::SqlId`] (refused
/// before the handler) or [`super::try_id_to_sql`] (an error instead of `i32::MAX`).
#[must_use]
pub fn id_to_sql(id: u64) -> i32 {
    i32::try_from(id).unwrap_or(i32::MAX)
}

/// Convertit un identifiant SQL `INT4` en identifiant d'API (`u64`) ; une valeur négative donne `0`.
#[must_use]
pub fn id_from_sql(id: i32) -> u64 {
    u64::try_from(id).unwrap_or_default()
}

// Le trait que l'API va implémenter sur ses DTOs
pub trait ApiRequestDto: DeserializeOwned {
    fn query_sql(&self) -> &'static str;
    fn query_params(&self) -> &[QueryParam];
    fn cache_key(&self) -> Option<String> {
        None
    }
    fn cache_ttl(&self) -> Option<u64> {
        None
    }
}

fn build_arguments(params: &[QueryParam]) -> Result<PgArguments, DbError> {
    let mut args = PgArguments::default();

    for param in params {
        match param {
            QueryParam::I32(v) => {
                args.add(*v).map_err(|e| DbError::Internal(e.to_string()))?;
            }
            QueryParam::I64(v) => {
                args.add(*v).map_err(|e| DbError::Internal(e.to_string()))?;
            }
            QueryParam::Text(v) => {
                args.add(v.clone())
                    .map_err(|e| DbError::Internal(e.to_string()))?;
            }
            QueryParam::Bool(v) => {
                args.add(*v).map_err(|e| DbError::Internal(e.to_string()))?;
            }
            QueryParam::Uuid(v) => {
                args.add(*v).map_err(|e| DbError::Internal(e.to_string()))?;
            }
            QueryParam::DateTime(v) => {
                args.add(*v).map_err(|e| DbError::Internal(e.to_string()))?;
            }
            QueryParam::IpAddr(v) => {
                args.add(*v).map_err(|e| DbError::Internal(e.to_string()))?;
            }
            QueryParam::OptionI32(v) => {
                args.add(*v).map_err(|e| DbError::Internal(e.to_string()))?;
            }
        }
    }

    Ok(args)
}

#[derive(Clone)]
pub struct Database {
    inner: Arc<DatabaseInner>,
}

struct DatabaseInner {
    database_url: String,
    pool: Mutex<Option<PgPool>>,
}

impl Database {
    pub async fn new(database_url: &str) -> Self {
        Self {
            inner: Arc::new(DatabaseInner {
                database_url: database_url.to_string(),
                pool: match PgPool::connect(database_url).await {
                    Ok(pool) => Mutex::new(Some(pool)),
                    Err(e) => {
                        tracing::error!(error = %e, "Failed to connect to the database");
                        Mutex::new(None)
                    }
                },
            }),
        }
    }

    pub async fn is_connected(&self) -> bool {
        self.inner.pool.lock().await.is_some()
    }

    async fn get_pool(&self) -> Result<PgPool, DbError> {
        let mut guard = self.inner.pool.lock().await;
        if let Some(pool) = &*guard {
            return Ok(pool.clone());
        }

        // Connexion paresseuse si le pool est vide
        let pool = PgPool::connect(&self.inner.database_url)
            .await
            .map_err(|_| DbError::Sqlx(sqlx::Error::PoolClosed))?;

        *guard = Some(pool.clone());
        drop(guard);
        Ok(pool)
    }

    /// Exécute une requête d'écriture (`INSERT`, `UPDATE`, `DELETE`) sans lire de résultat.
    ///
    /// # Errors
    ///
    /// Renvoie une [`DbError`] si la connexion au pool échoue, si un paramètre ne peut pas être
    /// lié ou si Postgres rejette la requête.
    pub async fn execute<Q: ApiRequestDto>(&self, query: &Q) -> Result<(), DbError> {
        let pool = self.get_pool().await?;
        execute_on(&pool, query).await
    }

    /// L'API demande un seul résultat (équivalent à `fetch_one` de sqlx)
    ///
    /// # Errors
    ///
    /// Renvoie une [`DbError`] si la connexion au pool échoue, si un paramètre ne peut pas être
    /// lié ou si Postgres rejette la requête ; `DbError::NotFound` si aucune ligne
    /// n'est renvoyée et `DbError::MappingError` si le JSON ne correspond pas à `T`.
    pub async fn fetch_one<T, Q: ApiRequestDto>(&self, query: &Q) -> Result<T, DbError>
    where
        T: DeserializeOwned,
    {
        let pool = self.get_pool().await?;
        fetch_one_on(&pool, query).await
    }

    /// Renvoie toutes les lignes, chacune décodée depuis une colonne JSON.
    ///
    /// # Errors
    ///
    /// Renvoie une [`DbError`] si la connexion au pool échoue, si un paramètre ne peut pas être
    /// lié ou si Postgres rejette la requête ; `DbError::MappingError` si une ligne ne
    /// correspond pas à `T`.
    pub async fn fetch_all<T, Q: ApiRequestDto>(&self, query: &Q) -> Result<Vec<T>, DbError>
    where
        T: DeserializeOwned,
    {
        let pool = self.get_pool().await?;
        fetch_all_on(&pool, query).await
    }

    /// Renvoie une valeur scalaire unique décodée directement par sqlx.
    ///
    /// # Errors
    ///
    /// Renvoie une [`DbError`] si la connexion au pool échoue, si un paramètre ne peut pas être
    /// lié ou si Postgres rejette la requête ; `DbError::NotFound` si aucune ligne
    /// n'est renvoyée.
    pub async fn fetch_scalar<T, Q>(&self, query: &Q) -> Result<T, DbError>
    where
        Q: ApiRequestDto,
        T: for<'r> sqlx::Decode<'r, sqlx::Postgres> + sqlx::Type<sqlx::Postgres> + Send + Unpin,
    {
        let pool = self.get_pool().await?;
        fetch_scalar_on(&pool, query).await
    }

    /// Starts a transaction: the queries run through the returned [`DbTransaction`] are applied
    /// all together on [`DbTransaction::commit`], or not at all.
    ///
    /// # Errors
    ///
    /// Returns a [`DbError`] if no connection can be obtained or `BEGIN` fails.
    pub async fn begin(&self) -> Result<DbTransaction, DbError> {
        let pool = self.get_pool().await?;
        Ok(DbTransaction {
            tx: pool.begin().await?,
        })
    }
}

/// A Postgres transaction opened by [`Database::begin`].
///
/// Same query methods as [`Database`], all run on the transaction's connection. Dropping it
/// without calling [`Self::commit`] rolls it back (early return on `?` included).
pub struct DbTransaction {
    tx: sqlx::Transaction<'static, sqlx::Postgres>,
}

impl DbTransaction {
    /// # Errors
    ///
    /// Same as [`Database::execute`].
    pub async fn execute<Q: ApiRequestDto>(&mut self, query: &Q) -> Result<(), DbError> {
        execute_on(&mut *self.tx, query).await
    }

    /// # Errors
    ///
    /// Same as [`Database::fetch_one`].
    pub async fn fetch_one<T, Q: ApiRequestDto>(&mut self, query: &Q) -> Result<T, DbError>
    where
        T: DeserializeOwned,
    {
        fetch_one_on(&mut *self.tx, query).await
    }

    /// # Errors
    ///
    /// Same as [`Database::fetch_all`].
    pub async fn fetch_all<T, Q: ApiRequestDto>(&mut self, query: &Q) -> Result<Vec<T>, DbError>
    where
        T: DeserializeOwned,
    {
        fetch_all_on(&mut *self.tx, query).await
    }

    /// # Errors
    ///
    /// Same as [`Database::fetch_scalar`].
    pub async fn fetch_scalar<T, Q>(&mut self, query: &Q) -> Result<T, DbError>
    where
        Q: ApiRequestDto,
        T: for<'r> sqlx::Decode<'r, sqlx::Postgres> + sqlx::Type<sqlx::Postgres> + Send + Unpin,
    {
        fetch_scalar_on(&mut *self.tx, query).await
    }

    /// Applies every query of the transaction.
    ///
    /// # Errors
    ///
    /// Returns a [`DbError`] if `COMMIT` fails (e.g. a deferred constraint); nothing is applied.
    pub async fn commit(self) -> Result<(), DbError> {
        self.tx.commit().await.map_err(DbError::from)
    }

    /// Discards every query of the transaction (same as dropping it, but awaited).
    ///
    /// # Errors
    ///
    /// Returns a [`DbError`] if `ROLLBACK` fails.
    pub async fn rollback(self) -> Result<(), DbError> {
        self.tx.rollback().await.map_err(DbError::from)
    }
}

// --- Queries on any executor: the pool, or the connection of a transaction ---

async fn execute_on<'e, E, Q>(executor: E, query: &Q) -> Result<(), DbError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    Q: ApiRequestDto,
{
    let args = build_arguments(query.query_params())?;
    sqlx::query_with(sqlx::AssertSqlSafe(query.query_sql()), args)
        .execute(executor)
        .await?;
    Ok(())
}

async fn fetch_one_on<'e, E, T, Q>(executor: E, query: &Q) -> Result<T, DbError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    T: DeserializeOwned,
    Q: ApiRequestDto,
{
    let args = build_arguments(query.query_params())?;
    let json_val: serde_json::Value =
        sqlx::query_scalar_with(sqlx::AssertSqlSafe(query.query_sql()), args)
            .fetch_one(executor)
            .await?;
    // Serde transforme le JSON directement dans le DTO de l'API
    serde_json::from_value(json_val).map_err(|e| DbError::MappingError(e.to_string()))
}

async fn fetch_all_on<'e, E, T, Q>(executor: E, query: &Q) -> Result<Vec<T>, DbError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    T: DeserializeOwned,
    Q: ApiRequestDto,
{
    let args = build_arguments(query.query_params())?;
    // Récupère une liste de valeurs JSON (une par ligne)
    let json_values: Vec<serde_json::Value> =
        sqlx::query_scalar_with(sqlx::AssertSqlSafe(query.query_sql()), args)
            .fetch_all(executor)
            .await?;
    json_values
        .into_iter()
        .map(|json_val| {
            serde_json::from_value(json_val).map_err(|e| DbError::MappingError(e.to_string()))
        })
        .collect()
}

async fn fetch_scalar_on<'e, E, T, Q>(executor: E, query: &Q) -> Result<T, DbError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    Q: ApiRequestDto,
    T: for<'r> sqlx::Decode<'r, sqlx::Postgres> + sqlx::Type<sqlx::Postgres> + Send + Unpin,
{
    let args = build_arguments(query.query_params())?;
    let result = sqlx::query_scalar_with(sqlx::AssertSqlSafe(query.query_sql()), args)
        .fetch_one(executor)
        .await?;
    Ok(result)
}
