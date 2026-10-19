use opentelemetry::trace::TracerProvider as _;
use opentelemetry_sdk::trace::SdkTracerProvider;
use tracing::Subscriber;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::{EnvFilter, Layer};

/// Filter of the trace layer: `sqlx::query` is a `DEBUG` target, everything else stays at `INFO`,
/// except actix's request log (request line and client address), kept out of the spans.
const TRACE_FILTER: &str = "info,sqlx::query=debug,actix_web::middleware::logger=off";

/// Trace layer bridging `tracing` spans to OpenTelemetry through `provider`.
#[must_use]
pub fn trace_layer<S>(provider: &SdkTracerProvider) -> impl Layer<S>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    tracing_opentelemetry::layer()
        .with_tracer(provider.tracer(env!("CARGO_PKG_NAME")))
        .with_filter(EnvFilter::new(TRACE_FILTER))
}
