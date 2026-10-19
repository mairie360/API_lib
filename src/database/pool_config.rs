use std::fmt::Display;
use std::str::FromStr;
use std::time::Duration;

use sqlx::postgres::PgPoolOptions;

use crate::env_manager::get_env_var;

/// Connections the pool keeps open even when idle, per process (MAIR-565).
pub const DB_POOL_MIN_CONNECTIONS_ENV: &str = "DB_POOL_MIN_CONNECTIONS";

/// Maximum number of connections of the pool, per process (MAIR-565).
pub const DB_POOL_MAX_CONNECTIONS_ENV: &str = "DB_POOL_MAX_CONNECTIONS";

/// Seconds after which an idle connection above the minimum is closed (MAIR-565).
pub const DB_POOL_IDLE_TIMEOUT_SECS_ENV: &str = "DB_POOL_IDLE_TIMEOUT_SECS";

/// [`DB_POOL_MIN_CONNECTIONS_ENV`] when unset or invalid: one warm connection, so the first
/// request after a pause does not pay for a new Postgres connection (sqlx's default is `0`).
pub const DEFAULT_DB_POOL_MIN_CONNECTIONS: u32 = 1;

/// [`DB_POOL_MAX_CONNECTIONS_ENV`] when unset or invalid: sqlx's own default.
pub const DEFAULT_DB_POOL_MAX_CONNECTIONS: u32 = 10;

/// [`DB_POOL_IDLE_TIMEOUT_SECS_ENV`] when unset or invalid: sqlx's own default (10 min).
pub const DEFAULT_DB_POOL_IDLE_TIMEOUT: Duration = Duration::from_mins(10);

/// Sizing of the Postgres pool of [`Database`](super::db_interface::Database).
///
/// Every replica of every API of an instance shares Postgres' `max_connections`: size
/// `max_connections` against it, and `min_connections` to what the instance's idle traffic needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DbPoolConfig {
    /// Connections kept open even when idle; never above `max_connections`.
    pub min_connections: u32,
    /// Connections the pool may open at most; always `>= 1`.
    pub max_connections: u32,
    /// Idle connections above `min_connections` are closed after this delay.
    pub idle_timeout: Duration,
}

impl Default for DbPoolConfig {
    fn default() -> Self {
        Self {
            min_connections: DEFAULT_DB_POOL_MIN_CONNECTIONS,
            max_connections: DEFAULT_DB_POOL_MAX_CONNECTIONS,
            idle_timeout: DEFAULT_DB_POOL_IDLE_TIMEOUT,
        }
    }
}

impl DbPoolConfig {
    /// Reads [`DB_POOL_MIN_CONNECTIONS_ENV`], [`DB_POOL_MAX_CONNECTIONS_ENV`] and
    /// [`DB_POOL_IDLE_TIMEOUT_SECS_ENV`]. An unset variable takes its default; an invalid one
    /// (not an integer, `0` for the maximum or the idle timeout) takes its default with a
    /// warning, and a minimum above the maximum is lowered to the maximum with a warning.
    #[must_use]
    pub fn from_env() -> Self {
        let max_connections = read_at_least(
            DB_POOL_MAX_CONNECTIONS_ENV,
            DEFAULT_DB_POOL_MAX_CONNECTIONS,
            1,
        );
        let mut min_connections = read_at_least(
            DB_POOL_MIN_CONNECTIONS_ENV,
            DEFAULT_DB_POOL_MIN_CONNECTIONS,
            0,
        );
        if min_connections > max_connections {
            tracing::warn!(
                min_connections,
                max_connections,
                "{DB_POOL_MIN_CONNECTIONS_ENV} is above {DB_POOL_MAX_CONNECTIONS_ENV}, using the maximum"
            );
            min_connections = max_connections;
        }
        let idle_timeout = Duration::from_secs(read_at_least(
            DB_POOL_IDLE_TIMEOUT_SECS_ENV,
            DEFAULT_DB_POOL_IDLE_TIMEOUT.as_secs(),
            1,
        ));
        Self {
            min_connections,
            max_connections,
            idle_timeout,
        }
    }

    /// The sqlx options of this configuration.
    #[must_use]
    pub fn pool_options(&self) -> PgPoolOptions {
        PgPoolOptions::new()
            .min_connections(self.min_connections.min(self.max_connections))
            .max_connections(self.max_connections)
            .idle_timeout(self.idle_timeout)
    }
}

/// `name` parsed as an integer `>= min`, else `default` (with a warning when the variable is set).
fn read_at_least<T>(name: &str, default: T, min: T) -> T
where
    T: FromStr + PartialOrd + Display + Copy,
{
    let Some(raw) = get_env_var(name) else {
        return default;
    };
    match raw.trim().parse::<T>() {
        Ok(value) if value >= min => value,
        _ => {
            tracing::warn!(
                value = %raw,
                default = %default,
                "{name} is not an integer >= {min}, using the default"
            );
            default
        }
    }
}
