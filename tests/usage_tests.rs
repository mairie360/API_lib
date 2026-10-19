//! Usage telemetry without identifiers (MAIR-501).
use std::time::{Duration, UNIX_EPOCH};

use actix_web::test as actix_test;
use actix_web::{middleware, web, App, HttpMessage, HttpRequest, HttpResponse};
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::usage::{
    is_allowed_span_attribute, usage_metrics, usage_middleware, UsageLedger, USAGE_METRICS_PATH,
};

const PERIOD: Duration = Duration::from_mins(1);

#[test]
fn only_actions_are_allowed_on_spans() {
    assert!(is_allowed_span_attribute("http.route"));
    assert!(is_allowed_span_attribute("http.response.status_code"));
    for forbidden in [
        "user.id",
        "enduser.id",
        "http.target",
        "url.query",
        "http.request.body",
        "client.address",
        "user_agent.original",
    ] {
        assert!(!is_allowed_span_attribute(forbidden), "{forbidden}");
    }
}

#[test]
fn distinct_users_are_counted_and_small_counts_are_not_exported() {
    let ledger = UsageLedger::with_settings("project-api", PERIOD, 5, 10);
    let t0 = UNIX_EPOCH + Duration::from_mins(100);
    // 6 users list the projects, one of them twice.
    for user in 1..=6 {
        ledger.record(
            "/api/v1/projects",
            "GET",
            200,
            Duration::from_millis(10),
            Some(user),
            t0,
        );
    }
    ledger.record(
        "/api/v1/projects",
        "GET",
        200,
        Duration::from_millis(10),
        Some(3),
        t0,
    );
    // A rare operation by 2 users: under k = 5.
    ledger.record(
        "/api/v1/projects/{id}",
        "DELETE",
        204,
        Duration::from_millis(5),
        Some(8),
        t0,
    );
    ledger.record(
        "/api/v1/projects/{id}",
        "DELETE",
        204,
        Duration::from_millis(5),
        Some(9),
        t0,
    );

    assert!(
        ledger.closed_entries(t0).is_empty(),
        "the open period is never exported"
    );
    let entries = ledger.closed_entries(t0 + PERIOD);
    assert_eq!(entries.len(), 1, "{entries:?}");
    assert_eq!(entries[0].operation, "/api/v1/projects");
    assert_eq!(entries[0].actions, 7);
    assert_eq!(entries[0].distinct_users, 6);
    // The rare operation is in no entry, and the `other` bucket (2 actions) is under k too.
    assert!(entries
        .iter()
        .all(|e| e.operation != "/api/v1/projects/{id}" && e.operation != "other"));
}

#[test]
fn the_export_holds_no_identifier() {
    let ledger = UsageLedger::with_settings("core-api", PERIOD, 1, 10);
    let t0 = UNIX_EPOCH + Duration::from_mins(100);
    ledger.record(
        "/api/v1/user/{id}/",
        "GET",
        200,
        Duration::from_millis(3),
        Some(987_654_321),
        t0,
    );
    let text = ledger.render_prometheus(t0 + PERIOD);
    assert!(
        text.contains(
            "mairie360_usage_distinct_users{service=\"core-api\",operation=\"/api/v1/user/{id}/\""
        ),
        "{text}"
    );
    assert!(!text.contains("987654321"), "no user id: {text}");
    assert!(
        !text
            .split(|c: char| !c.is_ascii_hexdigit())
            .any(|w| w.len() >= 32),
        "no hash: {text}"
    );
}

async fn item(req: HttpRequest) -> HttpResponse {
    // Stands for JwtMiddleware, which puts the authenticated user in the extensions.
    req.extensions_mut()
        .insert(AuthenticatedUser { id: 424_242 });
    HttpResponse::Ok().finish()
}

#[actix_web::test]
async fn the_middleware_records_the_route_template_not_the_path() {
    let ledger = web::Data::new(UsageLedger::with_settings(
        "test-api",
        Duration::from_secs(1),
        1,
        10,
    ));
    let app = actix_test::init_service(
        App::new()
            .app_data(ledger.clone())
            .wrap(middleware::from_fn(usage_middleware))
            .route("/items/{id}", web::get().to(item))
            .route(USAGE_METRICS_PATH, web::get().to(usage_metrics)),
    )
    .await;
    let req = actix_test::TestRequest::get()
        .uri("/items/123?search=jean.dupont%40example.com")
        .to_request();
    assert!(actix_test::call_service(&app, req)
        .await
        .status()
        .is_success());
    // Let the one-second period close.
    std::thread::sleep(Duration::from_millis(1100));
    let body = actix_test::call_and_read_body(
        &app,
        actix_test::TestRequest::get()
            .uri(USAGE_METRICS_PATH)
            .to_request(),
    )
    .await;
    let text = String::from_utf8(body.to_vec()).unwrap();
    assert!(
        text.contains("operation=\"/items/{id}\",method=\"GET\",status=\"200\""),
        "{text}"
    );
    assert!(
        !text.contains("123") || !text.contains("/items/123"),
        "{text}"
    );
    assert!(
        !text.contains("dupont") && !text.contains("424242"),
        "{text}"
    );
}
