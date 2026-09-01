pub mod redis;
use crate::database::db_interface::Database;
use deadpool_redis::{Config, Pool, Runtime};

pub struct AppState {
    redis_pool: Option<Pool>,
    pub db_interface: Database,
}

impl AppState {
    pub async fn new(redis_url: String, pg_url: String) -> Self {
        // --- Initialisation Redis ---
        let redis_cfg = Config::from_url(redis_url);
        let redis_pool = redis_cfg.create_pool(Some(Runtime::Tokio1));

        // --- Initialisation PostgreSQL ---
        let db_interface = Database::new(&pg_url).await;

        eprintln!("redis status: {:?}", redis_pool.is_ok());
        eprintln!("pg status: {:?}", db_interface.is_connected().await);

        Self {
            redis_pool: match redis_pool {
                Ok(pool) => Some(pool),
                Err(_) => None,
            },
            db_interface: db_interface,
        }
    }

    pub async fn get_redis_conn(&self) -> Option<deadpool_redis::Connection> {
        match &self.redis_pool {
            Some(pool) => pool.get().await.ok(),
            None => None,
        }
    }

    pub fn get_db_interface(&self) -> &Database {
        &self.db_interface
    }
}
