//! Logs and traces shared by every API (MAIR-503): the log format of Core API (coloured text, or
//! JSON with `LOG_FORMAT=json`), one exported span per request carrying its route, status, the
//! continued `traceparent` and the SQL it ran, and neither logs nor spans carrying the client
//! address or the query string (MAIR-290).

use std::io::Write;
use std::sync::{Arc, Mutex};

use actix_web::test::TestRequest;
use actix_web::{middleware, web, App, HttpResponse};
use mairie360_api_lib::logging::log_layer;
use mairie360_api_lib::logging::telemetry::opentelemetry::{global, Value};
use mairie360_api_lib::logging::telemetry::opentelemetry_sdk::propagation::TraceContextPropagator;
use mairie360_api_lib::logging::telemetry::opentelemetry_sdk::trace::{
    InMemorySpanExporter, SdkTracerProvider, SimpleSpanProcessor, SpanData,
};
use mairie360_api_lib::logging::telemetry::{trace_layer, tracer_provider, TracingLogger};
use mairie360_api_lib::security::JwtMiddleware;
use mairie360_api_lib::test_setup::http_setup::{authorization_for, test_app_state, SeededUser};
use serial_test::serial;
use tracing::subscriber::DefaultGuard;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::layer::SubscriberExt;

const SERVICE_NAME: &str = "telemetry-test";
const ROUTE: &str = "/api/v1/projects/";
const SPAN_NAME: &str = "GET /api/v1/projects/";

/// Collects what a log layer writes.
#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<u8>>>);

impl Captured {
    fn text(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
}

impl Write for Captured {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'writer> MakeWriter<'writer> for Captured {
    type Writer = Self;

    fn make_writer(&'writer self) -> Self::Writer {
        self.clone()
    }
}

/// Logs of `f` written by `log_layer`, built with `LOG_FORMAT` set to `format` and no `NO_COLOR`.
fn logs_with_format(format: Option<&str>, f: impl FnOnce()) -> String {
    let logs = Captured::default();
    let layer = temp_env::with_vars(
        [
            ("LOG_FORMAT", format),
            ("NO_COLOR", None),
            ("RUST_LOG", None),
        ],
        || log_layer(logs.clone()),
    );
    tracing::subscriber::with_default(tracing_subscriber::registry().with(layer), f);
    logs.text()
}

#[test]
#[serial]
fn text_logs_are_coloured_and_name_their_target() {
    let logs = logs_with_format(
        None,
        || tracing::info!(target: "project_api", "server started"),
    );
    assert!(logs.contains("server started"), "{logs}");
    assert!(
        logs.contains("project_api"),
        "the target is printed: {logs}"
    );
    assert!(
        logs.contains('\u{1b}'),
        "text logs are coloured like Core API's, even when not written to a terminal: {logs:?}"
    );
}

#[test]
#[serial]
fn json_logs_are_one_object_per_line() {
    let logs = logs_with_format(Some("json"), || {
        tracing::warn!(target: "project_api", "rate limit disabled");
    });
    let line: serde_json::Value = serde_json::from_str(logs.trim()).expect("one JSON object");
    assert_eq!(line["level"], "WARN");
    assert_eq!(line["target"], "project_api");
    assert_eq!(line["fields"]["message"], "rate limit disabled");
    assert!(
        !logs.contains('\u{1b}'),
        "no colour codes in JSON: {logs:?}"
    );
}

#[test]
#[serial]
fn debug_logs_are_hidden_by_default() {
    let logs = logs_with_format(None, || tracing::debug!("noise"));
    assert!(logs.is_empty(), "{logs}");
}

/// Provider exporting to memory through the redaction, and the subscriber feeding it, installed
/// for the current thread.
fn in_memory_tracing() -> (InMemorySpanExporter, SdkTracerProvider, DefaultGuard) {
    global::set_text_map_propagator(TraceContextPropagator::new());
    let exporter = InMemorySpanExporter::default();
    let provider = tracer_provider(SimpleSpanProcessor::new(exporter.clone()), SERVICE_NAME);
    let guard = tracing::subscriber::set_default(
        tracing_subscriber::registry().with(trace_layer(&provider)),
    );
    (exporter, provider, guard)
}

fn request_span(exporter: &InMemorySpanExporter, provider: &SdkTracerProvider) -> SpanData {
    provider.force_flush().expect("flush the spans");
    let spans = exporter.get_finished_spans().unwrap();
    spans
        .iter()
        .find(|span| span.name == SPAN_NAME)
        .cloned()
        .unwrap_or_else(|| panic!("no span named {SPAN_NAME}: {spans:#?}"))
}

fn attribute<'a>(span: &'a SpanData, key: &str) -> Option<&'a Value> {
    span.attributes
        .iter()
        .find(|kv| kv.key.as_str() == key)
        .map(|kv| &kv.value)
}

async fn handler() -> HttpResponse {
    tracing::error!("the handler logs an error");
    HttpResponse::Ok().finish()
}

/// Same mounting as the APIs: `TracingLogger` on the app, `JwtMiddleware` on `/api`.
macro_rules! traced_api {
    () => {
        actix_web::test::init_service(
            App::new()
                .wrap(TracingLogger::default())
                .app_data(test_app_state().await)
                .service(
                    web::scope("/api")
                        .wrap(JwtMiddleware)
                        .route("/v1/projects/", web::get().to(handler)),
                ),
        )
        .await
    };
}

#[actix_web::test]
#[serial]
async fn a_request_is_exported_as_a_span_with_its_sql() {
    let (exporter, provider, _guard) = in_memory_tracing();
    let app = traced_api!();
    let authorization = authorization_for(SeededUser::Alice).await;
    exporter.reset();

    let trace_id = "4bf92f3577b34da6a3ce929d0e0e4736";
    let request = TestRequest::get()
        .uri(&format!("{ROUTE}?search=Dupont"))
        .insert_header(authorization)
        .insert_header(("traceparent", format!("00-{trace_id}-00f067aa0ba902b7-01")))
        .to_request();
    let response = actix_web::test::call_service(&app, request).await;
    assert!(response.status().is_success(), "{}", response.status());
    // The root span closes with the response body: drop it before reading the exported spans.
    drop(response);

    let span = request_span(&exporter, &provider);
    assert_eq!(
        span.span_context.trace_id().to_string(),
        trace_id,
        "the incoming traceparent must be continued"
    );
    assert_eq!(attribute(&span, "http.status_code"), Some(&Value::I64(200)));
    assert_eq!(attribute(&span, "http.route"), Some(&Value::from(ROUTE)));
    assert_eq!(
        attribute(&span, "http.target"),
        Some(&Value::from(ROUTE)),
        "the query string must not be exported"
    );
    assert_eq!(attribute(&span, "http.client_ip"), None);
    assert_eq!(
        attribute(&span, "service.name"),
        None,
        "service.name belongs to the resource, not to the span"
    );

    // `JwtMiddleware` checks the account in Postgres: `sqlx` logs the statement as a
    // `sqlx::query` event, its values stay bound parameters.
    let has_sql = span.events.iter().any(|event| {
        let has = |key: &str| event.attributes.iter().any(|kv| kv.key.as_str() == key);
        has("db.statement")
            && event
                .attributes
                .iter()
                .any(|kv| kv.key.as_str() == "target" && kv.value == Value::from("sqlx::query"))
    });
    assert!(
        has_sql,
        "the SQL run during the request must be attached to its span: {:#?}",
        span.events
    );
}

#[actix_web::test]
#[serial]
async fn a_refused_request_still_gets_its_span() {
    let (exporter, provider, _guard) = in_memory_tracing();
    let app = traced_api!();
    exporter.reset();

    // No token: `JwtMiddleware` refuses before the handler runs.
    let _ =
        actix_web::test::try_call_service(&app, TestRequest::get().uri(ROUTE).to_request()).await;

    let span = request_span(&exporter, &provider);
    assert_eq!(attribute(&span, "http.status_code"), Some(&Value::I64(401)));
}

/// No database: the logs and spans of a request carrying personal data in its query string and
/// client address, mounted like the APIs (`Logger`, then `TracingLogger`).
#[actix_web::test]
#[serial]
async fn neither_logs_nor_spans_carry_the_query_string_or_the_client_address() {
    global::set_text_map_propagator(TraceContextPropagator::new());
    let exporter = InMemorySpanExporter::default();
    let provider = tracer_provider(SimpleSpanProcessor::new(exporter.clone()), SERVICE_NAME);
    let logs = Captured::default();
    let log_layer = temp_env::with_vars([("LOG_FORMAT", None::<&str>), ("RUST_LOG", None)], || {
        log_layer(logs.clone())
    });
    let _guard = tracing::subscriber::set_default(
        tracing_subscriber::registry()
            .with(log_layer)
            .with(trace_layer(&provider)),
    );
    let app = actix_web::test::init_service(
        App::new()
            .wrap(middleware::Logger::default())
            .wrap(TracingLogger::default())
            .route(ROUTE, web::get().to(handler)),
    )
    .await;

    let request = TestRequest::get()
        .uri(&format!("{ROUTE}?search=Dupont"))
        .insert_header(("x-forwarded-for", "203.0.113.7"))
        .to_request();
    let response = actix_web::test::call_service(&app, request).await;
    drop(response);
    provider.force_flush().expect("flush the spans");

    let logs = logs.text();
    let event = logs
        .lines()
        .find(|line| line.contains("the handler logs an error"))
        .unwrap_or_else(|| panic!("the handler's event must be logged: {logs}"));
    assert!(
        !event.contains("Dupont"),
        "query string in the logs: {event}"
    );
    assert!(
        !event.contains("203.0.113.7"),
        "client address in the logs: {event}"
    );

    let spans = format!("{:?}", exporter.get_finished_spans().unwrap());
    assert!(spans.contains("the handler logs an error"), "{spans}");
    assert!(
        !spans.contains("Dupont"),
        "query string in the spans: {spans}"
    );
    assert!(
        !spans.contains("203.0.113.7"),
        "client address in the spans: {spans}"
    );
}
