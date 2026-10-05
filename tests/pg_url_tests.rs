//! `build_pg_url` (MAIR-427, moved from the APIs) and `AppState::from_env`.

use mairie360_api_lib::database::build_pg_url;
use percent_encoding::percent_decode_str;
use url::Url;

fn decode(component: &str) -> String {
    percent_decode_str(component)
        .decode_utf8()
        .expect("component must be valid UTF-8")
        .into_owned()
}

#[test]
fn plain_password_is_unchanged() {
    assert_eq!(
        build_pg_url("postgres", "postgres", "db", "5432", "postgres"),
        "postgres://postgres:postgres@db:5432/postgres"
    );
}

#[test]
fn reserved_characters_are_encoded() {
    assert_eq!(
        build_pg_url("user", "a/b@c:d#e%f g", "db", "5432", "postgres"),
        "postgres://user:a%2Fb%40c%3Ad%23e%25f%20g@db:5432/postgres"
    );
}

/// Parses the URL the way sqlx does (`url::Url`, then percent-decoding of the
/// user, password and database) and checks every component comes back intact.
#[test]
fn components_round_trip_through_url_parsing() {
    let password = "p/@:#% ?&=+[]\\x";
    let url = build_pg_url("app user", password, "db.internal", "6543", "mairie db");
    let parsed = Url::parse(&url).expect("the URL must be valid");

    assert_eq!(parsed.scheme(), "postgres");
    assert_eq!(decode(parsed.username()), "app user");
    assert_eq!(parsed.password().map(decode).as_deref(), Some(password));
    assert_eq!(parsed.host_str(), Some("db.internal"));
    assert_eq!(parsed.port(), Some(6543));
    assert_eq!(decode(parsed.path().trim_start_matches('/')), "mairie db");
}

#[test]
fn base64_password_with_slash_round_trips() {
    // Shape of the generated secrets that broke the first dry-run deployment.
    let password = "not/a+real/secret==";
    let url = build_pg_url("postgres", password, "db", "5432", "postgres");
    let parsed = Url::parse(&url).expect("the URL must be valid");

    assert_eq!(parsed.port(), Some(5432));
    assert_eq!(parsed.password().map(decode).as_deref(), Some(password));
}

/// `AppState::from_env` assembles the same URL from `DB_*` and connects with it. Only test of
/// this binary that touches the process environment.
#[tokio::test]
async fn app_state_from_env_connects_with_the_db_variables() {
    use mairie360_api_lib::state::AppState;
    use mairie360_api_lib::test_setup::queries_setup::get_shared_db;

    let (_db, shared_url) = get_shared_db().await;
    let parsed = Url::parse(shared_url).unwrap();
    std::env::set_var("JWT_SECRET", "test-only-pg-url-secret-0123456789abcdef");
    std::env::set_var("JWT_TIMEOUT", "3600");
    std::env::set_var("REDIS_URL", "redis://127.0.0.1:1");
    std::env::set_var("DB_USER", decode(parsed.username()));
    std::env::set_var("DB_PASSWORD", parsed.password().map(decode).unwrap());
    std::env::set_var("DB_HOST", parsed.host_str().unwrap());
    std::env::set_var("DB_PORT", parsed.port().unwrap().to_string());
    std::env::set_var("DB_NAME", decode(parsed.path().trim_start_matches('/')));

    let state = AppState::from_env().await;
    assert!(state.readiness().await.postgres);
}
