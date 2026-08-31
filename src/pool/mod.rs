use crate::{database::db_interface::Database, redis::redis_interface::Redis};

pub struct AppState {
    redis_interface: Redis,
    db_interface: Database,
}

impl AppState {
    pub async fn new(redis_url: String, pg_url: String) -> Self {
        // --- Initialisation Redis ---
        let redis_interface = Redis::new(&redis_url);

        // --- Initialisation PostgreSQL ---
        let db_interface = Database::new(&pg_url).await;

        eprintln!("redis status: {:?}", redis_interface.is_connected().await);
        eprintln!("pg status: {:?}", db_interface.is_connected().await);

        Self {
            redis_interface,
            db_interface,
        }
    }

    pub async fn get_redis_conn(&self) -> &Redis {
        &self.redis_interface
    }

    pub fn get_db_interface(&self) -> &Database {
        &self.db_interface
    }
}
