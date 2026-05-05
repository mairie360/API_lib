pub mod redis;
use deadpool_redis::{Config, Pool, Runtime};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub struct AppState {
    redis_pool: Option<Pool>,
    pub db_pool: Option<PgPool>,
}

impl AppState {
    pub async fn new(redis_url: String, pg_url: String) -> Self {
        // --- Initialisation Redis ---
        let redis_cfg = Config::from_url(redis_url);
        let redis_pool = redis_cfg.create_pool(Some(Runtime::Tokio1));

        // --- Initialisation PostgreSQL ---
        let db_pool = PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(std::time::Duration::from_secs(3))
            .connect(&pg_url)
            .await;

        eprintln!("redis status: {:?}", redis_pool.is_ok());
        eprintln!("pg status: {:?}", db_pool.is_ok());

        Self {
            redis_pool: match redis_pool {
                Ok(pool) => Some(pool),
                Err(_) => None,
            },
            db_pool: match db_pool {
                Ok(pool) => Some(pool),
                Err(_) => None,
            },
        }
    }

    pub async fn get_redis_conn(&self) -> Option<deadpool_redis::Connection> {
        match &self.redis_pool {
            Some(pool) => pool.get().await.ok(),
            None => None,
        }
    }

    pub async fn get_db_conn(&self) -> Option<sqlx::pool::PoolConnection<sqlx::Postgres>> {
        match &self.db_pool {
            Some(pool) => pool.acquire().await.ok(),
            None => None,
        }
    }
}
