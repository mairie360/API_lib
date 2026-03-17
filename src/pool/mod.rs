pub mod redis;
use deadpool_redis::{Config, Pool, Runtime};
// use sqlx::PgPool;

pub struct AppState {
    redis_pool: Pool,
    // pub db_pool: PgPool,
}

impl AppState {
    /// Initialise le pool de manière asynchrone
    pub async fn new(redis_url: String) -> Self {
        let cfg = Config::from_url(redis_url);
        let pool = cfg
            .create_pool(Some(Runtime::Tokio1))
            .expect("Failed to create Redis pool");

        Self { redis_pool: pool }
    }

    pub async fn get_redis_conn(&self) -> deadpool_redis::Connection {
        self.redis_pool.get().await.unwrap()
    }
}
