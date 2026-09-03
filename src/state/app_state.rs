use crate::{
    database::db_interface::Database, redis::redis_interface::Redis, smart_db::SmartDatabase,
};

pub struct AppState {
    smart_db: SmartDatabase,
    redis: Redis,
}

impl AppState {
    pub async fn new(redis_url: String, pg_url: String) -> Self {
        // --- Initialisation Redis ---
        let redis_interface = Redis::new(&redis_url);

        // --- Initialisation PostgreSQL ---
        let db_interface = Database::new(&pg_url).await;

        println!("redis status: {:?}", redis_interface.is_connected().await);
        println!("pg status: {:?}", db_interface.is_connected().await);

        // `Redis` encapsule un `Arc` interne : le clone partage le même pool et le
        // même état de connexion que celui détenu par la `SmartDatabase`, les deux
        // restent donc synchronisés.
        let smart_db = SmartDatabase::new(db_interface, redis_interface.clone());

        Self {
            smart_db,
            redis: redis_interface,
        }
    }

    pub fn get_smart_db(&self) -> &SmartDatabase {
        &self.smart_db
    }

    pub fn get_redis(&self) -> &Redis {
        &self.redis
    }
}
