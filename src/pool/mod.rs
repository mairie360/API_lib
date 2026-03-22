pub mod redis;
use deadpool_redis::{Config, Pool, Runtime};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub struct AppState {
    redis_pool: Pool,
    pub db_pool: PgPool,
}

impl AppState {
    pub async fn new(redis_url: String, pg_url: String) -> Self {
        // --- Initialisation Redis ---
        let redis_cfg = Config::from_url(redis_url);
        let redis_pool = redis_cfg
            .create_pool(Some(Runtime::Tokio1))
            .expect("Failed to create Redis pool");

        // --- Initialisation PostgreSQL ---
        let db_pool = PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(std::time::Duration::from_secs(3))
            .connect(&pg_url)
            .await
            .expect("Failed to create Postgres pool");

        Self {
            redis_pool,
            db_pool,
        }
    }

    pub async fn get_redis_conn(&self) -> deadpool_redis::Connection {
        self.redis_pool.get().await.unwrap()
    }

    pub async fn get_db_conn(&self) -> sqlx::pool::PoolConnection<sqlx::Postgres> {
        self.db_pool.acquire().await.unwrap()
    }
}
