use tracing_subscriber::{fmt, EnvFilter};

use crate::env_manager::get_env_var;

/// Set to `json` for one JSON object per log line (production, log collectors); anything else
/// or unset gives the human-readable format.
pub const LOG_FORMAT_ENV: &str = "LOG_FORMAT";

/// Filter used when `RUST_LOG` is not set.
const DEFAULT_FILTER: &str = "info";

/// Installs the process-wide `tracing` subscriber that prints the logs of the lib and of the API
/// (MAIR-421): level filter from `RUST_LOG` (default `info`), format from [`LOG_FORMAT_ENV`].
///
/// Call it first thing in `main`. An API that installs its own subscriber (`Core_API` and its
/// OpenTelemetry layer) does not need it; calling it anyway is harmless.
///
/// Does nothing when a subscriber is already installed.
pub fn init_logging() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_FILTER));
    let json = get_env_var(LOG_FORMAT_ENV).is_some_and(|v| v.trim().eq_ignore_ascii_case("json"));
    let builder = fmt().with_env_filter(filter).with_target(true);
    // `try_init` fails only when a subscriber is already installed: keep that one.
    let _ = if json {
        builder.json().try_init()
    } else {
        builder.try_init()
    };
}
