//! MAIR-680: the middlewares accept the session tokens Core signs with its Ed25519 key and refuse
//! forged ones, on a service that holds only the public key and no `JWT_SECRET` any more (the end
//! of the transition). Own binary: the variables are set once for the whole process.
use actix_web::{http::StatusCode, test, web, App, HttpResponse};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use mairie360_api_lib::jwt_manager::{
    authenticate_token, generate_session_jwt, parse_public_keys, parse_signing_key, Claims,
    SESSION_JWT_AUDIENCE, SESSION_JWT_ISSUER,
};
use mairie360_api_lib::security::JwtMiddleware;
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::state::AppState;
use mairie360_api_lib::test_setup::http_setup::{
    status_of, SeededUser, TEST_JWT_PRIVATE_KEY, TEST_JWT_PUBLIC_KEY,
};
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;

/// An attacker's key pair: never in `JWT_PUBLIC_KEY`.
const ATTACKER_PRIVATE_KEY: &str = "-----BEGIN PRIVATE KEY-----
MC4CAQAwBQYDK2VwBCIEII1DTYiu+dcwBgtEKil59uTH8s7XfrxGrbD+/P0sXq/8
-----END PRIVATE KEY-----
";
const ATTACKER_PUBLIC_KEY: &str = "-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEAtZHz1BNkijjkLJlnK9l7vikDl0aivrmbwllpT68Lnss=
-----END PUBLIC KEY-----
";

static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    // Core's key pair; no `JWT_SECRET`: the historical HS256 tokens are over.
    std::env::set_var("JWT_PUBLIC_KEY", TEST_JWT_PUBLIC_KEY);
    std::env::set_var("JWT_PRIVATE_KEY", TEST_JWT_PRIVATE_KEY);
    std::env::remove_var("JWT_SECRET");
    std::env::set_var("JWT_TIMEOUT", "3600");
});

fn setup() {
    std::sync::LazyLock::force(&INIT);
}

async fn smart_db() -> SmartDatabase {
    let (_container, url) = get_shared_db().await;
    AppState::new(String::new(), url.clone())
        .await
        .get_smart_db()
        .clone()
}

fn in_one_hour() -> usize {
    usize::try_from(chrono::Utc::now().timestamp()).unwrap() + 3600
}

/// An Admin token for `user_id` signed with the attacker's key, claiming Core's `kid`.
fn forged_admin_token(user_id: u64) -> String {
    let attacker = parse_signing_key(
        ATTACKER_PRIVATE_KEY,
        &parse_public_keys(ATTACKER_PUBLIC_KEY).unwrap(),
    )
    .unwrap();
    let mut header = Header::new(Algorithm::EdDSA);
    header.kid = Some(
        parse_public_keys(TEST_JWT_PUBLIC_KEY).unwrap()[0]
            .kid()
            .to_string(),
    );
    let claims = Claims::new(&user_id.to_string(), "Admin", in_one_hour())
        .with_issuer_and_audience(SESSION_JWT_ISSUER, SESSION_JWT_AUDIENCE);
    encode(&header, &claims, attacker.encoding_key()).unwrap()
}

/// An HS256 Admin token signed with the old shared secret.
fn legacy_admin_token(user_id: u64) -> String {
    encode(
        &Header::new(Algorithm::HS256),
        &Claims::new(&user_id.to_string(), "Admin", in_one_hour()),
        &EncodingKey::from_secret(b"0123456789abcdef0123456789abcdef"),
    )
    .unwrap()
}

#[tokio::test]
async fn mair_680_a_core_eddsa_token_authenticates() {
    setup();
    let alice = SeededUser::Alice.id().await;
    let db = smart_db().await;
    let token = generate_session_jwt(&alice.to_string(), "Admin", None).unwrap();
    assert_eq!(
        authenticate_token(&token, &db, None).await.ok(),
        Some(alice)
    );
}

#[tokio::test]
async fn mair_680_forged_and_legacy_tokens_are_refused() {
    setup();
    let alice = SeededUser::Alice.id().await;
    let db = smart_db().await;
    for (case, token) in [
        ("signed with another key", forged_admin_token(alice)),
        ("HS256 with the old secret", legacy_admin_token(alice)),
    ] {
        assert!(
            authenticate_token(&token, &db, None).await.is_err(),
            "{case}"
        );
    }
}

#[tokio::test]
async fn mair_680_jwt_middleware_answers_401_to_a_forged_admin_token() {
    setup();
    let alice = SeededUser::Alice.id().await;
    let (_container, url) = get_shared_db().await;
    let state = web::Data::new(AppState::new(String::new(), url.clone()).await);
    let app = test::init_service(
        App::new()
            .app_data(state)
            .wrap(JwtMiddleware)
            .route("/protected", web::get().to(HttpResponse::Ok)),
    )
    .await;

    let genuine = generate_session_jwt(&alice.to_string(), "Admin", None).unwrap();
    for (token, expected) in [
        (genuine, StatusCode::OK),
        (forged_admin_token(alice), StatusCode::UNAUTHORIZED),
        (legacy_admin_token(alice), StatusCode::UNAUTHORIZED),
    ] {
        let req = test::TestRequest::get()
            .uri("/protected")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        assert_eq!(status_of(&app, req).await, expected);
    }
}
