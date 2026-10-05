//! Per-caller rate limiting (MAIR-425).

use std::num::NonZeroU32;

use actix_web::{
    http::{header::RETRY_AFTER, StatusCode},
    middleware::from_fn,
    test as actix_test, web, App, HttpMessage, HttpResponse,
};
use mairie360_api_lib::security::{
    rate_limit_middleware, AuthenticatedUser, RateLimitConfig, RateLimiter,
};

const fn config(burst: u32, trust_forwarded: bool) -> RateLimitConfig {
    RateLimitConfig {
        per_second: NonZeroU32::MIN,
        burst: NonZeroU32::new(burst).unwrap(),
        trust_forwarded,
    }
}

async fn ok() -> HttpResponse {
    HttpResponse::Ok().finish()
}

macro_rules! app {
    ($limiter:expr) => {
        actix_test::init_service(
            App::new()
                .app_data($limiter)
                .wrap(from_fn(rate_limit_middleware))
                .route("/", web::get().to(ok)),
        )
        .await
    };
}

fn from_peer(ip: &str) -> actix_test::TestRequest {
    actix_test::TestRequest::get()
        .uri("/")
        .peer_addr(format!("{ip}:1234").parse().unwrap())
}

#[actix_web::test]
async fn a_caller_over_its_burst_gets_429_with_retry_after() {
    let app = app!(RateLimiter::new(config(2, false)));
    for _ in 0..2 {
        let res = actix_test::call_service(&app, from_peer("10.0.0.1").to_request()).await;
        assert_eq!(res.status(), StatusCode::OK);
    }
    let res = actix_test::call_service(&app, from_peer("10.0.0.1").to_request()).await;
    assert_eq!(res.status(), StatusCode::TOO_MANY_REQUESTS);
    let retry_after: u64 = res
        .headers()
        .get(RETRY_AFTER)
        .unwrap()
        .to_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!(retry_after >= 1);

    // Another address has its own budget.
    let res = actix_test::call_service(&app, from_peer("10.0.0.2").to_request()).await;
    assert_eq!(res.status(), StatusCode::OK);
}

#[actix_web::test]
async fn authenticated_requests_are_counted_per_user_not_per_address() {
    let app = app!(RateLimiter::new(config(1, false)));
    // Same BFF address, two users: one budget each.
    for user in [1, 2] {
        let req = from_peer("10.0.0.9").to_request();
        req.extensions_mut().insert(AuthenticatedUser { id: user });
        assert_eq!(
            actix_test::call_service(&app, req).await.status(),
            StatusCode::OK
        );
    }
    let req = from_peer("10.0.0.9").to_request();
    req.extensions_mut().insert(AuthenticatedUser { id: 1 });
    assert_eq!(
        actix_test::call_service(&app, req).await.status(),
        StatusCode::TOO_MANY_REQUESTS
    );
}

#[actix_web::test]
async fn forwarded_addresses_count_only_when_trusted() {
    let forwarded = |client: &str| {
        from_peer("10.0.0.5")
            .insert_header(("X-Forwarded-For", client.to_string()))
            .to_request()
    };

    let trusted = app!(RateLimiter::new(config(1, true)));
    assert_eq!(
        actix_test::call_service(&trusted, forwarded("1.1.1.1"))
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        actix_test::call_service(&trusted, forwarded("2.2.2.2"))
            .await
            .status(),
        StatusCode::OK
    );

    let untrusted = app!(RateLimiter::new(config(1, false)));
    assert_eq!(
        actix_test::call_service(&untrusted, forwarded("1.1.1.1"))
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        actix_test::call_service(&untrusted, forwarded("2.2.2.2"))
            .await
            .status(),
        StatusCode::TOO_MANY_REQUESTS,
        "a spoofed header does not buy a new budget"
    );
}

#[actix_web::test]
async fn without_a_limiter_requests_pass() {
    let app = actix_test::init_service(
        App::new()
            .wrap(from_fn(rate_limit_middleware))
            .route("/", web::get().to(ok)),
    )
    .await;
    for _ in 0..5 {
        let res = actix_test::call_service(&app, from_peer("10.0.0.1").to_request()).await;
        assert_eq!(res.status(), StatusCode::OK);
    }
}

#[test]
fn config_falls_back_to_the_defaults() {
    temp_env::with_vars(
        [
            ("RATE_LIMIT_PER_SECOND", Some("0")),
            ("RATE_LIMIT_BURST", Some("abc")),
            ("RATE_LIMIT_TRUST_FORWARDED", None),
        ],
        || {
            let config = RateLimitConfig::from_env();
            assert_eq!(config.per_second.get(), 20);
            assert_eq!(config.burst.get(), 50);
            assert!(!config.trust_forwarded);
        },
    );
    temp_env::with_vars(
        [
            ("RATE_LIMIT_PER_SECOND", Some("5")),
            ("RATE_LIMIT_BURST", Some("8")),
            ("RATE_LIMIT_TRUST_FORWARDED", Some("TRUE")),
        ],
        || {
            assert_eq!(
                RateLimitConfig::from_env(),
                RateLimitConfig {
                    per_second: NonZeroU32::new(5).unwrap(),
                    burst: NonZeroU32::new(8).unwrap(),
                    trust_forwarded: true,
                }
            );
        },
    );
}
