//! OpenTelemetry tracing shared by every API (MAIR-503, from the Core API POC of MAIR-131).
//!
//! Traces are exported over OTLP/HTTP (protobuf) to a collector or agent (`OTel` Collector, Grafana
//! Alloy) that relays them to the observability backend (Scaleway Cockpit). Everything is opt-in
//! and driven by the standard OpenTelemetry environment variables, so a deployment without a
//! collector behaves exactly as before:
//!
//! - `OTEL_EXPORTER_OTLP_ENDPOINT` (or `OTEL_EXPORTER_OTLP_TRACES_ENDPOINT`): enables the export.
//!   With the generic variable, `/v1/traces` is appended, e.g. `http://alloy:4318`.
//! - `OTEL_SDK_DISABLED=true`: forces the export off even when an endpoint is set.
//! - `OTEL_SERVICE_NAME` (default given by the API to [`init_telemetry`]),
//!   `OTEL_RESOURCE_ATTRIBUTES`, `OTEL_EXPORTER_OTLP_HEADERS`, `OTEL_EXPORTER_OTLP_TIMEOUT`,
//!   `OTEL_TRACES_SAMPLER`: read by the SDK itself.
//! - `RUST_LOG` / `LOG_FORMAT`: the logs, printed whether the export is on or not (see
//!   [`super::log_layer`]).
//!
//! Each HTTP request gets a root span from [`TracingLogger`] (it continues the `traceparent` sent
//! by a BFF). The SQL statements run through `sqlx` are attached to it as span events
//! (`db.statement`, `elapsed`, rows) because `sqlx` reports them as `tracing` events of the
//! `sqlx::query` target, not as spans of their own.
//!
//! Spans never leave the process with personal data (MAIR-290, MAIR-501): [`Redact`] drops the
//! client address and the query string of `http.target` before the export, and the logs hide the
//! request root span.
//!
//! The `opentelemetry` crates are re-exported so the APIs (and their tests) use the exact versions
//! this crate was built with.

mod init_telemetry;
mod is_enabled;
mod redact;
mod telemetry_guard;
mod trace_layer;
mod tracer_provider;

pub use init_telemetry::init_telemetry;
pub use is_enabled::is_enabled;
pub use redact::Redact;
pub use telemetry_guard::TelemetryGuard;
pub use trace_layer::trace_layer;
pub use tracer_provider::tracer_provider;

pub use opentelemetry;
pub use opentelemetry_sdk;
pub use tracing_actix_web::TracingLogger;
