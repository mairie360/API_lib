use actix_web::middleware::Logger;

/// The format of [`request_logger`].
///
/// Client address, method, path (`%U`, without the query string), status, body size, duration in
/// seconds. No `Referer`: it carries the query of the front page that made the call.
pub const REQUEST_LOG_FORMAT: &str = r#"%a "%{method}xi %U" %s %b %T"#;

/// actix's `Logger` with [`REQUEST_LOG_FORMAT`]: wrap the app with it instead of
/// `Logger::default()`, whose `%r` logs the query string (`?search=dupont`).
#[must_use]
pub fn request_logger() -> Logger {
    Logger::new(REQUEST_LOG_FORMAT).custom_request_replace("method", |req| req.method().to_string())
}

#[cfg(test)]
mod tests {
    use super::request_logger;
    use actix_web::test as actix_test;
    use actix_web::{get, App, HttpRequest, HttpResponse};
    use std::sync::{Mutex, OnceLock};

    /// Collects what actix's `Logger` writes through the `log` facade.
    struct Captured(Mutex<Vec<String>>);

    impl log::Log for Captured {
        fn enabled(&self, _: &log::Metadata<'_>) -> bool {
            true
        }
        fn log(&self, record: &log::Record<'_>) {
            self.0.lock().unwrap().push(record.args().to_string());
        }
        fn flush(&self) {}
    }

    fn captured() -> &'static Captured {
        static CAPTURED: OnceLock<&'static Captured> = OnceLock::new();
        CAPTURED.get_or_init(|| {
            let captured: &'static Captured = Box::leak(Box::new(Captured(Mutex::new(Vec::new()))));
            log::set_logger(captured).expect("no other logger in the unit tests");
            log::set_max_level(log::LevelFilter::Info);
            captured
        })
    }

    #[get("/search")]
    async fn search(req: HttpRequest) -> HttpResponse {
        HttpResponse::Ok().body(req.query_string().to_string())
    }

    #[actix_web::test]
    async fn the_request_log_holds_no_query_string() {
        let captured = captured();
        let app = actix_test::init_service(App::new().wrap(request_logger()).service(search)).await;
        let req = actix_test::TestRequest::get()
            .uri("/search?search=gdpr.marker%40example.com")
            .insert_header(("Referer", "http://front/users?search=gdpr.marker"))
            .to_request();
        let body = actix_test::call_and_read_body(&app, req).await;
        assert_eq!(
            body, "search=gdpr.marker%40example.com",
            "the handler still reads the query"
        );

        let lines = captured.0.lock().unwrap().join("\n");
        assert!(lines.contains("\"GET /search\" 200"), "{lines}");
        assert!(!lines.contains("gdpr.marker"), "{lines}");
    }
}
