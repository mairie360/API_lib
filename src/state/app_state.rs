use std::time::Duration;

use super::readiness::{within_timeout, Readiness};
use crate::{
    database::{build_pg_url, db_interface::Database},
    env_manager::{get_critical_env_var, get_env_var},
    jwt_manager::enforce_jwt_config,
    keycloak::{KeycloakConfig, KeycloakTokenVerifier},
    redis::redis_interface::Redis,
    smart_db::SmartDatabase,
};

/// Seconds the API waits for Postgres at startup before giving up (MAIR-423).
pub const DB_CONNECT_TIMEOUT_ENV: &str = "DB_CONNECT_TIMEOUT";

/// [`DB_CONNECT_TIMEOUT_ENV`] when unset or not a positive integer.
pub const DEFAULT_DB_CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

fn db_connect_timeout() -> Duration {
    get_env_var(DB_CONNECT_TIMEOUT_ENV)
        .and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|seconds| *seconds > 0)
        .map_or(DEFAULT_DB_CONNECT_TIMEOUT, Duration::from_secs)
}

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

    /// [`AppState::new`] with the URLs read from the environment (MAIR-427), the block every
    /// API's `main.rs` used to copy: `REDIS_URL`, and `DB_USER`, `DB_PASSWORD`, `DB_HOST`,
    /// `DB_PORT`, `DB_NAME` assembled by [`build_pg_url`].
    ///
    /// # Panics
    ///
    /// When one of these variables is unset (see [`get_critical_env_var`]), and in the cases of
    /// [`AppState::with_keycloak`].
    pub async fn from_env() -> Self {
        let redis_url = get_critical_env_var("REDIS_URL");
        let pg_url = build_pg_url(
            &get_critical_env_var("DB_USER"),
            &get_critical_env_var("DB_PASSWORD"),
            &get_critical_env_var("DB_HOST"),
            &get_critical_env_var("DB_PORT"),
            &get_critical_env_var("DB_NAME"),
        );
        Self::new(redis_url, pg_url).await
    }

    /// Same as [`AppState::new`] with an explicit Keycloak configuration (`None` disables
    /// Keycloak tokens), regardless of the environment.
    ///
    /// # Panics
    ///
    /// - when `JWT_SECRET` / `JWT_TIMEOUT` are missing or weak, see [`enforce_jwt_config`]
    ///   (only logged in builds with the `test-utils` feature);
    /// - when Postgres cannot be reached within [`DB_CONNECT_TIMEOUT_ENV`] seconds (default
    ///   30): an API without its database must crash (and be restarted by Kubernetes), not start
    ///   and answer `500` to every request. Redis stays optional (the cache is skipped).
    pub async fn with_keycloak(
        redis_url: String,
        pg_url: String,
        keycloak: Option<KeycloakConfig>,
    ) -> Self {
        // Refuse to start with a missing or forgeable JWT configuration (MAIR-391).
        enforce_jwt_config();

        // --- Redis ---
        let redis_interface = Redis::new(&redis_url);

        // --- PostgreSQL: required, the API refuses to start without it (MAIR-423) ---
        let timeout = db_connect_timeout();
        let db_interface = match Database::connect_with_retry(&pg_url, timeout).await {
            Ok(db) => db,
            Err(error) => {
                tracing::error!(error = %error, "Postgres unreachable at startup");
                panic!("Postgres unreachable after {}s: {error}", timeout.as_secs());
            }
        };

        tracing::info!(
            connected = redis_interface.is_connected().await,
            "redis status"
        );
        tracing::info!(connected = db_interface.is_connected().await, "pg status");
        if let Some(config) = &keycloak {
            tracing::info!(
                "keycloak status: enabled (issuer {}, audiences {:?})",
                config.issuer(),
                config.audiences()
            );
        } else {
            tracing::info!("keycloak status: disabled (JWT_SECRET tokens only)");
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

    /// Checks Postgres (`SELECT 1`) and Redis (one round-trip) concurrently, each bounded by
    /// [`super::READINESS_CHECK_TIMEOUT`]. Serve [`Readiness::to_response`] on `GET /ready`.
    pub async fn readiness(&self) -> Readiness {
        let db = self.smart_db.get_db();
        let (postgres, redis) = tokio::join!(
            // Boxed: the two checks hold sqlx/deadpool futures of several kilobytes.
            Box::pin(within_timeout(db.ping())),
            Box::pin(within_timeout(self.redis.ping()))
        );
        Readiness { postgres, redis }
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
