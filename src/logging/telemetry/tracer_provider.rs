use opentelemetry_sdk::trace::{SdkTracerProvider, SpanProcessor};
use opentelemetry_sdk::Resource;

use super::Redact;

/// Tracer provider of the service: its resource (`service.name` from `OTEL_SERVICE_NAME`, else
/// `default_service_name`, plus `OTEL_RESOURCE_ATTRIBUTES`) and `processor` behind [`Redact`].
///
/// [`super::init_telemetry`] builds it with the OTLP batch exporter; tests pass an in-memory one.
#[must_use]
pub fn tracer_provider(
    processor: impl SpanProcessor + 'static,
    default_service_name: &'static str,
) -> SdkTracerProvider {
    let resource = if std::env::var_os("OTEL_SERVICE_NAME").is_some() {
        Resource::builder().build()
    } else {
        Resource::builder()
            .with_service_name(default_service_name)
            .build()
    };
    SdkTracerProvider::builder()
        .with_resource(resource)
        .with_span_processor(Redact(processor))
        .build()
}
