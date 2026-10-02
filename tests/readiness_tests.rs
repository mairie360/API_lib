//! Readiness checks and fail-fast startup (MAIR-423).

use std::time::{Duration, Instant};

use actix_web::{body::to_bytes, http::StatusCode};
use mairie360_api_lib::database::db_interface::Database;
use mairie360_api_lib::state::{AppState, Readiness, DB_CONNECT_TIMEOUT_ENV};
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use mairie360_api_lib::test_setup::redis_setup::start_redis_container;
use serial_test::serial;

/// Nothing listens on port 1.
const UNREACHABLE_PG: &str = "postgres://user:password@127.0.0.1:1/db";
const UNREACHABLE_REDIS: &str = "redis://127.0.0.1:1";

static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    std::env::set_var("JWT_SECRET", "test-only-readiness-secret-0123456789");
    std::env::set_var("JWT_TIMEOUT", "3600");
    std::env::set_var(DB_CONNECT_TIMEOUT_ENV, "1");
});

fn setup() {
    std::sync::LazyLock::force(&INIT);
}

async fn body_of(readiness: Readiness) -> (StatusCode, String) {
    let response = readiness.to_response();
    let status = response.status();
    let body = to_bytes(response.into_body()).await.unwrap();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

#[actix_web::test]
async fn the_response_names_the_dependencies_that_are_down() {
    let all_up = Readiness {
        postgres: true,
        redis: true,
    };
    assert_eq!(body_of(all_up).await, (StatusCode::OK, "ready".to_owned()));
    let redis_down = Readiness {
        postgres: true,
        redis: false,
    };
    assert_eq!(
        body_of(redis_down).await,
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "not ready: redis".to_owned()
        )
    );
    let all_down = Readiness {
        postgres: false,
        redis: false,
    };
    assert!(!all_down.is_ready());
    assert_eq!(
        body_of(all_down).await,
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "not ready: postgres, redis".to_owned()
        )
    );
}

#[tokio::test]
async fn connect_with_retry_gives_up_after_the_timeout() {
    let start = Instant::now();
    let result = Database::connect_with_retry(UNREACHABLE_PG, Duration::from_secs(2)).await;
    assert!(result.is_err());
    assert!(
        start.elapsed() < Duration::from_secs(5),
        "bounded by the timeout"
    );
}

#[tokio::test]
#[should_panic(expected = "Postgres unreachable")]
async fn the_api_refuses_to_start_without_its_database() {
    setup();
    AppState::new(UNREACHABLE_REDIS.to_string(), UNREACHABLE_PG.to_string()).await;
}

#[tokio::test]
#[serial]
async fn readiness_is_ok_when_both_dependencies_answer() {
    setup();
    let (_db, url) = get_shared_db().await;
    let (_redis_node, redis) = start_redis_container().await;
    let state = AppState::new(redis.url.clone(), url.clone()).await;
    assert_eq!(
        state.readiness().await,
        Readiness {
            postgres: true,
            redis: true
        }
    );
}

#[tokio::test]
#[serial]
async fn readiness_reports_redis_down_but_the_api_still_starts() {
    setup();
    let (_db, url) = get_shared_db().await;
    let state = AppState::new(UNREACHABLE_REDIS.to_string(), url.clone()).await;
    assert_eq!(
        state.readiness().await,
        Readiness {
            postgres: true,
            redis: false
        }
    );
}
