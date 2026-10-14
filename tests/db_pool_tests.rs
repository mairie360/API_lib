//! Postgres pool configurable per instance (MAIR-565): `DB_POOL_MIN_CONNECTIONS`,
//! `DB_POOL_MAX_CONNECTIONS` and `DB_POOL_IDLE_TIMEOUT_SECS`.

use std::time::Duration;

use mairie360_api_lib::database::db_interface::Database;
use mairie360_api_lib::database::{
    DbPoolConfig, DB_POOL_IDLE_TIMEOUT_SECS_ENV, DB_POOL_MAX_CONNECTIONS_ENV,
    DB_POOL_MIN_CONNECTIONS_ENV, DEFAULT_DB_POOL_IDLE_TIMEOUT, DEFAULT_DB_POOL_MAX_CONNECTIONS,
    DEFAULT_DB_POOL_MIN_CONNECTIONS,
};
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;

fn config_with(min: Option<&str>, max: Option<&str>, idle: Option<&str>) -> DbPoolConfig {
    temp_env::with_vars(
        [
            (DB_POOL_MIN_CONNECTIONS_ENV, min),
            (DB_POOL_MAX_CONNECTIONS_ENV, max),
            (DB_POOL_IDLE_TIMEOUT_SECS_ENV, idle),
        ],
        DbPoolConfig::from_env,
    )
}

#[test]
fn unset_variables_take_the_defaults() {
    let config = config_with(None, None, None);

    assert_eq!(config, DbPoolConfig::default());
    assert_eq!(config.min_connections, DEFAULT_DB_POOL_MIN_CONNECTIONS);
    assert_eq!(config.max_connections, DEFAULT_DB_POOL_MAX_CONNECTIONS);
    assert_eq!(config.idle_timeout, DEFAULT_DB_POOL_IDLE_TIMEOUT);
    // One warm connection, sqlx's maximum and idle timeout.
    assert_eq!(DEFAULT_DB_POOL_MIN_CONNECTIONS, 1);
    assert_eq!(DEFAULT_DB_POOL_MAX_CONNECTIONS, 10);
    assert_eq!(DEFAULT_DB_POOL_IDLE_TIMEOUT, Duration::from_secs(600));
}

#[test]
fn valid_values_configure_the_pool() {
    let config = config_with(Some("4"), Some(" 25 "), Some("120"));

    assert_eq!(
        config,
        DbPoolConfig {
            min_connections: 4,
            max_connections: 25,
            idle_timeout: Duration::from_secs(120),
        }
    );
    let options = config.pool_options();
    assert_eq!(options.get_min_connections(), 4);
    assert_eq!(options.get_max_connections(), 25);
    assert_eq!(options.get_idle_timeout(), Some(Duration::from_secs(120)));
}

#[test]
fn zero_minimum_is_allowed() {
    assert_eq!(config_with(Some("0"), None, None).min_connections, 0);
}

#[test]
fn invalid_values_fall_back_to_the_defaults() {
    for value in ["-5", "twenty", "", "1.5"] {
        let config = config_with(Some(value), Some(value), Some(value));
        assert_eq!(config, DbPoolConfig::default(), "value {value:?}");
    }
    // Overflows: connection counts are `u32`, the idle timeout is `u64` seconds.
    let config = config_with(
        Some("4294967296"),
        Some("4294967296"),
        Some("18446744073709551616"),
    );
    assert_eq!(config, DbPoolConfig::default());
    // `0` is a valid minimum, but neither a valid maximum nor a valid idle timeout.
    let config = config_with(None, Some("0"), Some("0"));
    assert_eq!(config, DbPoolConfig::default());
}

#[test]
fn minimum_above_the_maximum_is_lowered_to_the_maximum() {
    let config = config_with(Some("30"), Some("5"), None);

    assert_eq!(config.min_connections, 5);
    assert_eq!(config.max_connections, 5);
    // With the maximum unset, the minimum is lowered to the default maximum.
    assert_eq!(config_with(Some("50"), None, None).min_connections, 10);
}

/// The acceptance criterion: after a pause longer than the idle timeout, the pool still holds
/// `min_connections` open connections instead of reopening them on the next request.
#[tokio::test]
async fn min_connections_are_kept_after_a_pause() {
    let (_container, url) = get_shared_db().await;
    let config = DbPoolConfig {
        min_connections: 3,
        max_connections: 5,
        idle_timeout: Duration::from_secs(1),
    };
    let db = Database::connect_with_pool_config(url, Duration::from_secs(30), config)
        .await
        .expect("connect to the shared database");
    db.ping().await.expect("ping");

    // Several reaper passes (period = idle timeout) with no query at all.
    tokio::time::sleep(Duration::from_secs(4)).await;

    assert!(
        db.pool_size().await >= 3,
        "pool size {} after the pause",
        db.pool_size().await
    );
}

/// The behaviour MAIR-565 fixes: sqlx's default minimum (`0`) leaves no connection open after a
/// pause, so the first request pays for reconnecting.
#[tokio::test]
async fn without_minimum_the_pool_empties_after_a_pause() {
    let (_container, url) = get_shared_db().await;
    let config = DbPoolConfig {
        min_connections: 0,
        max_connections: 5,
        idle_timeout: Duration::from_secs(1),
    };
    let db = Database::connect_with_pool_config(url, Duration::from_secs(30), config)
        .await
        .expect("connect to the shared database");
    db.ping().await.expect("ping");

    tokio::time::sleep(Duration::from_secs(4)).await;

    assert_eq!(db.pool_size().await, 0);
}
