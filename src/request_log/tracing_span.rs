//! [`tracing_actix_web::TracingLogger`] without the values of the request: see the module
//! documentation of [`crate::request_log`].
//!
//! Wiring, `hide_query` outside the logger and `restore_query` inside it (the last `wrap` runs
//! first):
//!
//! ```ignore
//! App::new()
//!     .wrap(middleware::from_fn(restore_query))
//!     .wrap(TracingLogger::<RedactedRootSpanBuilder>::new())
//!     .wrap(middleware::from_fn(hide_query))
//! ```
//!
//! Disable the `emit_event_on_error` default feature of `tracing-actix-web` in the API
//! (`default-features = false`): it logs the `Debug` of every error, which
//! [`RedactedRootSpanBuilder`] replaces with its own event.

use crate::error::describe_json_error;
use actix_web::body::MessageBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::error::{JsonPayloadError, PathError, QueryPayloadError, UrlencodedError};
use actix_web::http::Uri;
use actix_web::middleware::Next;
use actix_web::{Error, HttpMessage};
use tracing::Span;
use tracing_actix_web::{DefaultRootSpanBuilder, RootSpanBuilder};

/// The full URI of a request whose query [`hide_query`] removed.
struct HiddenQuery(Uri);

/// Removes the query string from the request URI, for the root span built right after it (see the
/// module documentation). Wrap it **outside** `TracingLogger`, and [`restore_query`] inside.
///
/// # Errors
///
/// The errors of the inner services.
// actix-web runs each request on a single-threaded runtime per worker: the future holds the
// request (`Rc`) and does not need to be Send.
#[allow(clippy::future_not_send)]
pub async fn hide_query(
    mut req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    if req.uri().query().is_some() {
        let full = req.uri().clone();
        let mut parts = full.clone().into_parts();
        parts.path_and_query = parts.path_and_query.and_then(|pq| pq.path().parse().ok());
        if let Ok(path_only) = Uri::from_parts(parts) {
            req.extensions_mut().insert(HiddenQuery(full));
            req.head_mut().uri = path_only;
        }
    }
    next.call(req).await
}

/// Puts back the query string [`hide_query`] removed, before the handlers read it. Wrap it
/// **inside** `TracingLogger`.
///
/// # Errors
///
/// The errors of the inner services.
#[allow(clippy::future_not_send)]
pub async fn restore_query(
    mut req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    let hidden = req.extensions_mut().remove::<HiddenQuery>();
    if let Some(HiddenQuery(full)) = hidden {
        req.head_mut().uri = full;
    }
    next.call(req).await
}

/// Describes an error by its type and context, without the values it may quote.
///
/// The extractor errors of actix (JSON, query, path, form) lose their serde message or keep it
/// without the value; any other error is described by its `Display`, the text the client
/// receives.
#[must_use]
pub fn describe_error(error: &Error) -> String {
    if let Some(json) = error.as_error::<JsonPayloadError>() {
        return match json {
            JsonPayloadError::Deserialize(e) => {
                format!("Json deserialize error: {}", describe_json_error(e))
            }
            other => other.to_string(),
        };
    }
    if error.as_error::<QueryPayloadError>().is_some() {
        return "Query deserialize error".to_string();
    }
    if error.as_error::<PathError>().is_some() {
        return "Path deserialize error".to_string();
    }
    if error.as_error::<UrlencodedError>().is_some() {
        return "Urlencoded payload error".to_string();
    }
    error.to_string()
}

/// `TracingLogger` root span builder: the default span (path only, see [`hide_query`]), with
/// [`describe_error`] instead of the error's `Debug`.
pub struct RedactedRootSpanBuilder;

impl RootSpanBuilder for RedactedRootSpanBuilder {
    fn on_request_start(request: &ServiceRequest) -> Span {
        DefaultRootSpanBuilder::on_request_start(request)
    }

    fn on_request_end<B: MessageBody>(span: Span, outcome: &Result<ServiceResponse<B>, Error>) {
        let (status, error) = match outcome {
            Ok(response) => (response.status(), response.response().error()),
            Err(error) => (error.as_response_error().status_code(), Some(error)),
        };
        span.record("http.status_code", i32::from(status.as_u16()));
        let Some(error) = error else {
            span.record("otel.status_code", "OK");
            return;
        };
        let description = describe_error(error);
        span.record("exception.message", description.as_str());
        span.record("exception.details", description.as_str());
        if status.is_client_error() {
            span.record("otel.status_code", "OK");
            tracing::warn!(error = %description, "request refused");
        } else {
            span.record("otel.status_code", "ERROR");
            tracing::error!(error = %description, "request failed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{describe_error, hide_query, restore_query, RedactedRootSpanBuilder};
    use actix_web::test as actix_test;
    use actix_web::{get, middleware, post, web, App, HttpRequest, HttpResponse};
    use serde::Deserialize;
    use std::io::Write;
    use std::sync::{Arc, Mutex};
    use tracing_actix_web::TracingLogger;

    /// Collects what the `fmt` subscriber writes, to search it like the marker test does.
    #[derive(Clone, Default)]
    struct Captured(Arc<Mutex<Vec<u8>>>);

    impl Write for Captured {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl Captured {
        fn text(&self) -> String {
            String::from_utf8_lossy(&self.0.lock().unwrap()).into_owned()
        }
    }

    #[derive(Deserialize)]
    struct Body {
        #[allow(dead_code)]
        user_id: i32,
    }

    #[get("/search")]
    #[allow(clippy::future_not_send)]
    async fn search(req: HttpRequest) -> HttpResponse {
        tracing::warn!("searching");
        HttpResponse::BadRequest().body(req.query_string().to_string())
    }

    #[post("/typed")]
    async fn typed(_: web::Json<Body>) -> HttpResponse {
        HttpResponse::Ok().finish()
    }

    #[actix_web::test]
    async fn the_logs_hold_neither_the_query_nor_the_refused_value() {
        let captured = Captured::default();
        let writer = captured.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_ansi(false)
            .with_writer(move || writer.clone())
            .finish();
        let _guard = tracing::subscriber::set_default(subscriber);

        let app = actix_test::init_service(
            App::new()
                .wrap(middleware::from_fn(restore_query))
                .wrap(TracingLogger::<RedactedRootSpanBuilder>::new())
                .wrap(middleware::from_fn(hide_query))
                .service(search)
                .service(typed),
        )
        .await;

        let req = actix_test::TestRequest::get()
            .uri("/search?search=gdpr.marker%40example.com&page=2")
            .to_request();
        let body = actix_test::call_and_read_body(&app, req).await;
        // The handler still reads the query.
        assert_eq!(body, "search=gdpr.marker%40example.com&page=2");

        let req = actix_test::TestRequest::post()
            .uri("/typed")
            .set_json(serde_json::json!({ "user_id": "markertracervalue" }))
            .to_request();
        let response = actix_test::call_service(&app, req).await;
        assert_eq!(response.status(), 400);

        let logs = captured.text();
        assert!(logs.contains("http.target=/search"), "{logs}");
        assert!(logs.contains("searching"), "{logs}");
        assert!(logs.contains("Json deserialize error"), "{logs}");
        assert!(!logs.contains("gdpr.marker"), "{logs}");
        assert!(!logs.contains("page=2"), "{logs}");
        assert!(!logs.contains("markertracervalue"), "{logs}");
    }

    #[test]
    fn extractor_errors_lose_their_values() {
        let json: serde_json::Error = serde_json::from_str::<Body>(r#"{"user_id": "secret"}"#)
            .err()
            .unwrap();
        let error = actix_web::Error::from(actix_web::error::JsonPayloadError::Deserialize(json));
        let text = describe_error(&error);
        assert!(
            text.starts_with("Json deserialize error: invalid type: string"),
            "{text}"
        );
        assert!(!text.contains("secret"), "{text}");
        let error = actix_web::error::ErrorBadRequest("Invalid `email`: too long");
        assert_eq!(describe_error(&error), "Invalid `email`: too long");
    }
}
