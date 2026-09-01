use crate::{
    database::db_interface::Database, redis::redis_interface::Redis, smart_db::SmartDatabase,
};

pub struct AppState {
    smart_db: SmartDatabase,
}

impl AppState {
    pub async fn new(redis_url: String, pg_url: String) -> Self {
        // --- Initialisation Redis ---
        let redis_interface = Redis::new(&redis_url);

        // --- Initialisation PostgreSQL ---
        let db_interface = Database::new(&pg_url).await;

        println!("redis status: {:?}", redis_interface.is_connected().await);
        println!("pg status: {:?}", db_interface.is_connected().await);

        let smart_db = SmartDatabase::new(db_interface, redis_interface);

        Self { smart_db }
    }

    pub fn get_smart_db(&self) -> &SmartDatabase {
        &self.smart_db
    }
}
