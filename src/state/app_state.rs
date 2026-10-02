use crate::{
    database::db_interface::Database,
    jwt_manager::enforce_jwt_config,
    keycloak::{KeycloakConfig, KeycloakTokenVerifier},
    redis::redis_interface::Redis,
    smart_db::SmartDatabase,
};

pub struct AppState {
    smart_db: SmartDatabase,
    redis: Redis,
    keycloak: Option<KeycloakTokenVerifier>,
}

impl AppState {
    /// Builds the state with Keycloak read from the environment ([`KeycloakConfig::from_env`]):
    /// without `KEYCLOAK_REALM_URL` and `KEYCLOAK_CLIENT_ID`, the middlewares only accept the
    /// historical JWTs signed with `JWT_SECRET`.
    ///
    /// # Panics
    ///
    /// Same as [`AppState::with_keycloak`].
    pub async fn new(redis_url: String, pg_url: String) -> Self {
        Self::with_keycloak(redis_url, pg_url, KeycloakConfig::from_env()).await
    }

    /// Same as [`AppState::new`] with an explicit Keycloak configuration (`None` disables
    /// Keycloak tokens), regardless of the environment.
    ///
    /// # Panics
    ///
    /// Panics when `JWT_SECRET` / `JWT_TIMEOUT` are missing or weak, see
    /// [`enforce_jwt_config`] (only logged in builds with the `test-utils` feature).
    pub async fn with_keycloak(
        redis_url: String,
        pg_url: String,
        keycloak: Option<KeycloakConfig>,
    ) -> Self {
        // Refuse to start with a missing or forgeable JWT configuration (MAIR-391).
        enforce_jwt_config();

        // --- Redis ---
        let redis_interface = Redis::new(&redis_url);

        // --- PostgreSQL ---
        let db_interface = Database::new(&pg_url).await;

        println!("redis status: {:?}", redis_interface.is_connected().await);
        println!("pg status: {:?}", db_interface.is_connected().await);
        match &keycloak {
            Some(config) => println!(
                "keycloak status: enabled (issuer {}, audiences {:?})",
                config.issuer(),
                config.audiences()
            ),
            None => println!("keycloak status: disabled (JWT_SECRET tokens only)"),
        }

        // `Redis` encapsule un `Arc` interne : le clone partage le même pool et le
        // même état de connexion que celui détenu par la `SmartDatabase`, les deux
        // restent donc synchronisés.
        let smart_db = SmartDatabase::new(db_interface, redis_interface.clone());

        Self {
            smart_db,
            redis: redis_interface,
            keycloak: keycloak.map(KeycloakTokenVerifier::new),
        }
    }

    #[must_use]
    pub const fn get_smart_db(&self) -> &SmartDatabase {
        &self.smart_db
    }

    #[must_use]
    pub const fn get_redis(&self) -> &Redis {
        &self.redis
    }

    /// Verifier of Keycloak access tokens, `None` when Keycloak is not configured.
    #[must_use]
    pub const fn get_keycloak(&self) -> Option<&KeycloakTokenVerifier> {
        self.keycloak.as_ref()
    }
}
