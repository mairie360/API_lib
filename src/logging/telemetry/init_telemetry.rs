use opentelemetry::global;
use opentelemetry_otlp::{Protocol, SpanExporter, WithExportConfig};
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::BatchSpanProcessor;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use super::{is_enabled, trace_layer, tracer_provider, TelemetryGuard};
use crate::env_manager::get_env_var;
use crate::logging::log_layer;

/// Installs the process-wide subscriber: logs on stdout, plus the trace export when asked.
///
/// The logs are those of [`log_layer`]; the export is on when [`is_enabled`] says so.
/// `default_service_name` is the `service.name` reported when `OTEL_SERVICE_NAME` is not set
/// (e.g. `"project-api"`).
///
/// Call it first thing in `main` and keep the returned guard until exit (`None` without export).
/// Observability never takes the API down: when the exporter cannot be built, the reason is
/// logged and the API runs without traces. When a subscriber is already installed, it is kept
/// and nothing is exported.
#[must_use]
pub fn init_telemetry(default_service_name: &'static str) -> Option<TelemetryGuard> {
    let exporter = is_enabled(get_env_var).then(|| {
        SpanExporter::builder()
            .with_http()
            .with_protocol(Protocol::HttpBinary)
            .build()
    });
    let (provider, exporter_error) = match exporter {
        Some(Ok(exporter)) => (
            Some(tracer_provider(
                BatchSpanProcessor::builder(exporter).build(),
                default_service_name,
            )),
            None,
        ),
        Some(Err(error)) => (None, Some(error)),
        None => (None, None),
    };

    let traces = provider.as_ref().map(trace_layer);
    if let Err(error) = tracing_subscriber::registry()
        .with(log_layer(std::io::stdout))
        .with(traces)
        .try_init()
    {
        tracing::warn!("Trace export disabled: a tracing subscriber is already installed: {error}");
        return None;
    }
    if let Some(error) = exporter_error {
        tracing::warn!("OpenTelemetry disabled: cannot build the OTLP exporter: {error}");
    }
    let provider = provider?;
    global::set_tracer_provider(provider.clone());
    global::set_text_map_propagator(TraceContextPropagator::new());
    Some(TelemetryGuard::new(provider))
}
