use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use super::log_layer;

/// Installs the process-wide `tracing` subscriber printing the logs of the lib and the API.
///
/// Logs go through [`log_layer`] (MAIR-421): level filter from `RUST_LOG` (default `info`),
/// format from [`super::LOG_FORMAT_ENV`]. The `log` records (actix's `Logger`) are bridged to it.
///
/// Call it first thing in `main`. An API built with the `otel` feature calls
/// `logging::telemetry::init_telemetry` instead, which installs the same logs plus the trace
/// export; calling both is harmless.
///
/// Does nothing when a subscriber is already installed.
pub fn init_logging() {
    // `try_init` fails only when a subscriber is already installed: keep that one.
    let _ = tracing_subscriber::registry()
        .with(log_layer(std::io::stdout))
        .try_init();
}
