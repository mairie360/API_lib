mod init_logging;
mod log_layer;
/// OpenTelemetry trace export (MAIR-503), behind the `otel` feature.
#[cfg(feature = "otel")]
pub mod telemetry;

pub use init_logging::init_logging;
pub use log_layer::{log_layer, LOG_FORMAT_ENV};
