use std::time::Duration;

use opentelemetry::{Context, Key, KeyValue, Value};
use opentelemetry_sdk::error::OTelSdkResult;
use opentelemetry_sdk::trace::{Span, SpanData, SpanProcessor};
use opentelemetry_sdk::Resource;

/// Root span attributes that identify a person: dropped before the export.
const DROPPED_ATTRIBUTES: [&str; 1] = ["http.client_ip"];

/// Root span attribute holding the path and query string: only the path is exported.
const TARGET_ATTRIBUTE: &str = "http.target";

/// Span processor stripping the personal data of the spans (client address, query string)
/// before handing them to the wrapped processor (MAIR-290, MAIR-501).
#[derive(Debug)]
pub struct Redact<P>(pub P);

/// Removes the attributes of [`DROPPED_ATTRIBUTES`] and the query string of `http.target`.
fn redact(span: &mut SpanData) {
    span.attributes
        .retain(|kv| !DROPPED_ATTRIBUTES.contains(&kv.key.as_str()));
    for kv in &mut span.attributes {
        if kv.key.as_str() == TARGET_ATTRIBUTE {
            let target = kv.value.as_str();
            if let Some((path, _query)) = target.split_once('?') {
                *kv = KeyValue::new(
                    Key::from_static_str(TARGET_ATTRIBUTE),
                    Value::from(path.to_owned()),
                );
            }
        }
    }
}

impl<P: SpanProcessor> SpanProcessor for Redact<P> {
    fn on_start(&self, span: &mut Span, cx: &Context) {
        self.0.on_start(span, cx);
    }

    fn on_end(&self, mut span: SpanData) {
        redact(&mut span);
        self.0.on_end(span);
    }

    fn force_flush(&self) -> OTelSdkResult {
        self.0.force_flush()
    }

    fn shutdown_with_timeout(&self, timeout: Duration) -> OTelSdkResult {
        self.0.shutdown_with_timeout(timeout)
    }

    fn set_resource(&mut self, resource: &Resource) {
        self.0.set_resource(resource);
    }
}

#[cfg(test)]
mod tests {
    use super::redact;
    use opentelemetry::trace::{SpanContext, SpanId, SpanKind, Status};
    use opentelemetry::{InstrumentationScope, KeyValue, Value};
    use opentelemetry_sdk::trace::{SpanData, SpanEvents, SpanLinks};
    use std::time::SystemTime;

    fn span(attributes: Vec<KeyValue>) -> SpanData {
        SpanData {
            span_context: SpanContext::empty_context(),
            parent_span_id: SpanId::INVALID,
            parent_span_is_remote: false,
            span_kind: SpanKind::Server,
            name: "GET /api/v1/projects".into(),
            start_time: SystemTime::now(),
            end_time: SystemTime::now(),
            attributes,
            dropped_attributes_count: 0,
            events: SpanEvents::default(),
            links: SpanLinks::default(),
            status: Status::Unset,
            instrumentation_scope: InstrumentationScope::default(),
        }
    }

    #[test]
    fn redact_drops_the_client_address_and_the_query_string() {
        let mut data = span(vec![
            KeyValue::new("http.client_ip", "203.0.113.7"),
            KeyValue::new("http.target", "/api/v1/projects?search=Dupont"),
            KeyValue::new("http.route", "/api/v1/projects"),
        ]);
        redact(&mut data);
        let value = |key: &str| {
            data.attributes
                .iter()
                .find(|kv| kv.key.as_str() == key)
                .map(|kv| kv.value.clone())
        };
        assert_eq!(value("http.client_ip"), None);
        assert_eq!(value("http.target"), Some(Value::from("/api/v1/projects")));
        assert_eq!(value("http.route"), Some(Value::from("/api/v1/projects")));
    }

    #[test]
    fn redact_keeps_a_target_without_query_string() {
        let mut data = span(vec![KeyValue::new("http.target", "/api/v1/projects/3")]);
        redact(&mut data);
        assert_eq!(
            data.attributes,
            vec![KeyValue::new("http.target", "/api/v1/projects/3")]
        );
    }
}
