//! MAIR-264: tokens bound to a revoked session are refused by the middlewares, through the
//! shared Redis revocation list (`revoked:<sid>`), and MAIR-772: the session state `session:<sid>`
//! accepts the active token of a session and refuses the token a refresh replaced.
//!
//! Own test binary: `middlewares_tests` changes `JWT_TIMEOUT` while running.

use actix_web::{http::StatusCode, test, web, App, HttpResponse};
use mairie360_api_lib::jwt_manager::{
    decode_jwt, generate_bound_session_jwt, generate_jwt, generate_session_jwt, is_session_revoked,
    mark_session_revoked, publish_session_state, read_session_state, revoke_session,
    revoked_session_key, session_state_key, SessionBinding, SessionState, SessionStatus,
    REVOKED_SESSION_KEY_PREFIX,
};
use mairie360_api_lib::redis::redis_interface::Redis;
use mairie360_api_lib::security::{AdminMiddleware, JwtMiddleware};
use mairie360_api_lib::state::AppState;
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use mairie360_api_lib::test_setup::redis_setup::{get_redis_connection, start_redis_container};
use redis::Commands;
use serial_test::serial;

static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    std::env::set_var(
        "JWT_PUBLIC_KEY",
        mairie360_api_lib::test_setup::http_setup::TEST_JWT_PUBLIC_KEY,
    );
    std::env::set_var(
        "JWT_PRIVATE_KEY",
        mairie360_api_lib::test_setup::http_setup::test_jwt_private_key(),
    );
    std::env::set_var("JWT_TIMEOUT", "3600");
});

fn setup() {
    std::sync::LazyLock::force(&INIT);
}

/// Nothing listens on port 1: every Redis command fails right away.
const UNREACHABLE_REDIS: &str = "redis://127.0.0.1:1";

async fn index() -> HttpResponse {
    HttpResponse::Ok().body("Welcome!")
}

macro_rules! status_of {
    ($app:expr, $token:expr, $uri:expr) => {{
        let req = test::TestRequest::get()
            .uri($uri)
            .insert_header(("Authorization", format!("Bearer {}", $token)))
            .to_request();
        match test::try_call_service(&$app, req).await {
            Ok(res) => res.status(),
            Err(err) => err.as_response_error().status_code(),
        }
    }};
}

fn unique_sid() -> String {
    format!("test-session-{}", uuid::Uuid::new_v4())
}

// `actix_web::test` shadows the built-in `#[test]` attribute in this file.
#[core::prelude::v1::test]
fn key_prefix_is_exact() {
    assert_eq!(REVOKED_SESSION_KEY_PREFIX, "revoked:");
    assert_eq!(revoked_session_key("abc-123"), "revoked:abc-123");
}

// `actix_web::test` shadows the built-in `#[test]` attribute in this file.
#[core::prelude::v1::test]
fn sid_claim_round_trips_and_is_optional() {
    setup();
    let with_sid = generate_session_jwt("1", "", Some("abc-123")).unwrap();
    assert_eq!(decode_jwt(&with_sid).unwrap().session_id(), Some("abc-123"));

    let without_sid = generate_jwt("1", "").unwrap();
    assert_eq!(decode_jwt(&without_sid).unwrap().session_id(), None);
}

#[tokio::test]
#[serial]
async fn revoke_session_writes_the_exact_key_with_a_ttl() {
    let (_redis, config) = start_redis_container().await;
    let redis = Redis::new(&config.url);
    let sid = unique_sid();

    assert!(!is_session_revoked(&redis, &sid).await.unwrap());
    revoke_session(&redis, &sid, 120).await.unwrap();
    assert!(is_session_revoked(&redis, &sid).await.unwrap());

    // Checked with a raw connection: no prefix added by the crate.
    let mut conn = get_redis_connection(&config);
    let exists: bool = conn.exists(format!("revoked:{sid}")).unwrap();
    assert!(exists);
    let ttl: i64 = conn.ttl(format!("revoked:{sid}")).unwrap();
    assert!((1..=120).contains(&ttl), "ttl = {ttl}");

    // A TTL of 0 would be rejected by Redis: it is raised to 1 second.
    let other = unique_sid();
    revoke_session(&redis, &other, 0).await.unwrap();
    assert!(is_session_revoked(&redis, &other).await.unwrap());
}

#[tokio::test]
#[serial]
async fn jwt_middleware_refuses_a_revoked_session() {
    setup();
    let (_container, url) = get_shared_db().await;
    let (_redis, config) = start_redis_container().await;
    let app_state = web::Data::new(AppState::new(config.url.clone(), url.clone()).await);
    let app = test::init_service(
        App::new()
            .app_data(app_state.clone())
            .wrap(JwtMiddleware)
            .route("/protected", web::get().to(index)),
    )
    .await;

    let sid = unique_sid();
    let token = generate_session_jwt("1", "", Some(&sid)).unwrap();
    assert_eq!(status_of!(app, token, "/protected"), StatusCode::OK);

    revoke_session(app_state.get_redis(), &sid, 60)
        .await
        .unwrap();
    assert_eq!(
        status_of!(app, token, "/protected"),
        StatusCode::UNAUTHORIZED
    );

    // Another session of the same user is not affected.
    let other = generate_session_jwt("1", "", Some(&unique_sid())).unwrap();
    assert_eq!(status_of!(app, other, "/protected"), StatusCode::OK);
}

#[tokio::test]
#[serial]
async fn admin_middleware_refuses_a_revoked_session() {
    setup();
    let (_container, url) = get_shared_db().await;
    let (_redis, config) = start_redis_container().await;
    let app_state = web::Data::new(AppState::new(config.url.clone(), url.clone()).await);
    let app = test::init_service(
        App::new().app_data(app_state.clone()).service(
            web::scope("/api/v1/admin")
                .wrap(AdminMiddleware)
                .route("/all-users", web::get().to(index)),
        ),
    )
    .await;

    let sid = unique_sid();
    let token = generate_session_jwt("1", "Admin", Some(&sid)).unwrap();
    assert_eq!(
        status_of!(app, token, "/api/v1/admin/all-users"),
        StatusCode::OK
    );
    revoke_session(app_state.get_redis(), &sid, 60)
        .await
        .unwrap();
    assert_eq!(
        status_of!(app, token, "/api/v1/admin/all-users"),
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
#[serial]
async fn token_without_sid_is_accepted_even_without_redis() {
    setup();
    let (_container, url) = get_shared_db().await;
    let app_state = web::Data::new(AppState::new(UNREACHABLE_REDIS.to_string(), url.clone()).await);
    let app = test::init_service(
        App::new()
            .app_data(app_state.clone())
            .wrap(JwtMiddleware)
            .route("/protected", web::get().to(index)),
    )
    .await;

    let token = generate_jwt("1", "").unwrap();
    assert_eq!(status_of!(app, token, "/protected"), StatusCode::OK);
}

#[tokio::test]
#[serial]
async fn session_token_fails_closed_when_redis_is_down() {
    setup();
    let (_container, url) = get_shared_db().await;
    let app_state = web::Data::new(AppState::new(UNREACHABLE_REDIS.to_string(), url.clone()).await);
    let app = test::init_service(
        App::new()
            .app_data(app_state.clone())
            .wrap(JwtMiddleware)
            .route("/protected", web::get().to(index)),
    )
    .await;

    let token = generate_session_jwt("1", "", Some(&unique_sid())).unwrap();
    assert_eq!(
        status_of!(app, token, "/protected"),
        StatusCode::SERVICE_UNAVAILABLE
    );
}

macro_rules! protected_app {
    ($app_state:expr) => {
        test::init_service(
            App::new()
                .app_data($app_state.clone())
                .wrap(JwtMiddleware)
                .route("/protected", web::get().to(index)),
        )
        .await
    };
}

#[tokio::test]
#[serial]
async fn mair_772_the_session_state_accepts_the_active_token_and_refuses_the_replaced_one() {
    setup();
    let (_container, url) = get_shared_db().await;
    let (_redis, config) = start_redis_container().await;
    let app_state = web::Data::new(AppState::new(config.url.clone(), url.clone()).await);
    let app = protected_app!(app_state);

    let sid = unique_sid();
    let first = generate_session_jwt("1", "", Some(&sid)).unwrap();
    // A refresh: Core signs a new token and publishes its hash. Two tokens signed in the same second
    // can be identical, so the second one carries another role.
    let second = generate_session_jwt("1", "Admin", Some(&sid)).unwrap();
    assert_ne!(first, second);
    publish_session_state(
        app_state.get_redis(),
        &sid,
        &SessionState::active_with_token(1, &second, Some("92.184.10.1")),
        60,
    )
    .await
    .unwrap();

    assert_eq!(status_of!(app, second, "/protected"), StatusCode::OK);
    assert_eq!(
        status_of!(app, first, "/protected"),
        StatusCode::UNAUTHORIZED
    );

    // The key holds the hash, never the token, and expires with it.
    let mut conn = get_redis_connection(&config);
    let raw: String = conn.get(session_state_key(&sid)).unwrap();
    assert!(!raw.contains(&second));
    let ttl: i64 = conn.ttl(session_state_key(&sid)).unwrap();
    assert!((1..=60).contains(&ttl), "ttl = {ttl}");
}

#[tokio::test]
#[serial]
async fn mair_772_a_revoked_state_refuses_the_session_and_writes_the_revocation_list() {
    setup();
    let (_container, url) = get_shared_db().await;
    let (_redis, config) = start_redis_container().await;
    let app_state = web::Data::new(AppState::new(config.url.clone(), url.clone()).await);
    let app = protected_app!(app_state);

    let sid = unique_sid();
    let token = generate_session_jwt("1", "", Some(&sid)).unwrap();
    publish_session_state(
        app_state.get_redis(),
        &sid,
        &SessionState::active_with_token(1, &token, None),
        60,
    )
    .await
    .unwrap();
    assert_eq!(status_of!(app, token, "/protected"), StatusCode::OK);

    mark_session_revoked(app_state.get_redis(), &sid, 1, 60)
        .await
        .unwrap();
    assert_eq!(
        status_of!(app, token, "/protected"),
        StatusCode::UNAUTHORIZED
    );
    assert!(is_session_revoked(app_state.get_redis(), &sid)
        .await
        .unwrap());
    let state = read_session_state(app_state.get_redis(), &sid)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(state.state, SessionStatus::Revoked);
}

#[tokio::test]
#[serial]
async fn mair_772_a_state_of_another_account_is_refused() {
    setup();
    let (_container, url) = get_shared_db().await;
    let (_redis, config) = start_redis_container().await;
    let app_state = web::Data::new(AppState::new(config.url.clone(), url.clone()).await);
    let app = protected_app!(app_state);

    let sid = unique_sid();
    let token = generate_session_jwt("1", "", Some(&sid)).unwrap();
    publish_session_state(
        app_state.get_redis(),
        &sid,
        &SessionState::active_with_token(2, &token, None),
        60,
    )
    .await
    .unwrap();
    assert_eq!(
        status_of!(app, token, "/protected"),
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
#[serial]
async fn mair_772_a_session_unknown_to_redis_and_the_database_keeps_the_account_check() {
    setup();
    let (_container, url) = get_shared_db().await;
    let (_redis, config) = start_redis_container().await;
    let app_state = web::Data::new(AppState::new(config.url.clone(), url.clone()).await);
    let app = protected_app!(app_state);

    let sid = unique_sid();
    let token = generate_session_jwt("1", "", Some(&sid)).unwrap();
    assert_eq!(status_of!(app, token, "/protected"), StatusCode::OK);
    // Nothing is cached for a session the database does not know.
    assert!(read_session_state(app_state.get_redis(), &sid)
        .await
        .unwrap()
        .is_none());
}

// `actix_web::test` shadows the built-in `#[test]` attribute in this file.
#[core::prelude::v1::test]
fn mair_773_a_bound_session_token_carries_its_device_key_and_network() {
    setup();
    let binding = SessionBinding {
        device_key_thumbprint: Some("NzbLsXh8uDCcd-6MNwXF4W_7noWXFZAfHkxZsRGC9Xs".to_string()),
        network: Some("92.184.10.0/24".to_string()),
    };
    let token = generate_bound_session_jwt("1", "", Some("abc-123"), &binding).unwrap();
    let claims = decode_jwt(&token).unwrap();
    assert_eq!(
        claims.device_key_thumbprint(),
        Some("NzbLsXh8uDCcd-6MNwXF4W_7noWXFZAfHkxZsRGC9Xs")
    );
    assert_eq!(claims.network(), Some("92.184.10.0/24"));

    let unbound = decode_jwt(&generate_session_jwt("1", "", None).unwrap()).unwrap();
    assert_eq!(unbound.device_key_thumbprint(), None);
    assert_eq!(unbound.network(), None);
}
