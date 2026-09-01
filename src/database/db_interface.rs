use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use sqlx::postgres::PgArguments;
use sqlx::{Arguments, PgPool};
use std::net::IpAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("Erreur interne : {0}")]
    Internal(String),
    #[error("Erreur de correspondance du DTO : {0}")]
    MappingError(String),
    #[error("Erreur de base de données : {0}")]
    Sqlx(#[from] sqlx::Error),
}

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
    pub fn as_text(&self) -> &str {
        match self {
            QueryParam::Text(s) => s,
            _ => panic!("Expected Text, got {:?}", self),
        }
    }

    pub fn as_i32(&self) -> i32 {
        match self {
            QueryParam::I32(v) => *v,
            _ => panic!("Expected I32, got {:?}", self),
        }
    }

    pub fn as_i64(&self) -> i64 {
        match self {
            QueryParam::I64(v) => *v,
            _ => panic!("Expected I64, got {:?}", self),
        }
    }

    pub fn as_bool(&self) -> bool {
        match self {
            QueryParam::Bool(v) => *v,
            _ => panic!("Expected Bool, got {:?}", self),
        }
    }

    pub fn as_uuid(&self) -> Uuid {
        match self {
            QueryParam::Uuid(v) => *v,
            _ => panic!("Expected Uuid, got {:?}", self),
        }
    }

    pub fn as_datetime(&self) -> DateTime<Utc> {
        match self {
            QueryParam::DateTime(v) => *v,
            _ => panic!("Expected DateTime, got {:?}", self),
        }
    }

    pub fn as_ipaddr(&self) -> IpAddr {
        match self {
            QueryParam::IpAddr(v) => *v,
            _ => panic!("Expected IpAddr, got {:?}", self),
        }
    }

    pub fn as_option_i32(&self) -> Option<i32> {
        match self {
            QueryParam::OptionI32(v) => *v,
            _ => panic!("Expected OptionI32, got {:?}", self),
        }
    }
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
                args.add(v.clone())
                    .map_err(|e| DbError::Internal(e.to_string()))?;
            }
            QueryParam::DateTime(v) => {
                args.add(v.clone())
                    .map_err(|e| DbError::Internal(e.to_string()))?;
            }
            QueryParam::IpAddr(v) => {
                args.add(v.clone())
                    .map_err(|e| DbError::Internal(e.to_string()))?;
            }
            QueryParam::OptionI32(v) => {
                args.add(v.clone())
                    .map_err(|e| DbError::Internal(e.to_string()))?;
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
                        eprintln!("Failed to connect to database: {}", e);
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
        Ok(pool)
    }

    // Méthode interne partagée pour binder et exécuter la requête SQL brute
    pub async fn execute<Q: ApiRequestDto>(&self, query: &Q) -> Result<(), DbError> {
        let pool = self.get_pool().await?;
        let params = query.query_params();
        let args = build_arguments(&params)?;

        sqlx::query_with(sqlx::AssertSqlSafe(query.query_sql()), args)
            .execute(&pool)
            .await?;

        Ok(())
    }

    /// L'API demande un seul résultat (équivalent à fetch_one de sqlx)
    pub async fn fetch_one<T, Q: ApiRequestDto>(&self, query: &Q) -> Result<T, DbError>
    where
        T: DeserializeOwned,
    {
        let pool = self.get_pool().await?;

        let params = query.query_params();
        let args = build_arguments(&params)?;

        let json_val: serde_json::Value =
            sqlx::query_scalar_with(sqlx::AssertSqlSafe(query.query_sql()), args)
                .fetch_one(&pool)
                .await?;

        // Serde transforme le JSON directement dans le DTO de l'API
        let item: T =
            serde_json::from_value(json_val).map_err(|e| DbError::MappingError(e.to_string()))?;

        Ok(item)
    }

    pub async fn fetch_all<T, Q: ApiRequestDto>(&self, query: &Q) -> Result<Vec<T>, DbError>
    where
        T: DeserializeOwned,
    {
        let pool = self.get_pool().await?;

        let params = query.query_params();
        let args = build_arguments(&params)?;

        // Récupère une liste de valeurs JSON (une par ligne)
        let json_values: Vec<serde_json::Value> =
            sqlx::query_scalar_with(sqlx::AssertSqlSafe(query.query_sql()), args)
                .fetch_all(&pool)
                .await?;

        let mut items = Vec::new();
        for json_val in json_values {
            let item: T = serde_json::from_value(json_val)
                .map_err(|e| DbError::MappingError(e.to_string()))?;
            items.push(item);
        }

        Ok(items)
    }

    pub async fn fetch_scalar<T, Q>(&self, query: &Q) -> Result<T, DbError>
    where
        Q: ApiRequestDto,
        // Contraintes nécessaires pour que sqlx sache décoder un type scalaire (ex: bool, i64)
        T: for<'r> sqlx::Decode<'r, sqlx::Postgres> + sqlx::Type<sqlx::Postgres> + Send + Unpin,
    {
        let pool = self.get_pool().await?;
        let params = query.query_params();
        let args = build_arguments(&params)?;

        // Utilisation de query_scalar_with pour exécuter la requête avec les arguments dynamiques
        let result = sqlx::query_scalar_with(sqlx::AssertSqlSafe(query.query_sql()), args)
            .fetch_one(&pool)
            .await?;

        Ok(result)
    }
}
