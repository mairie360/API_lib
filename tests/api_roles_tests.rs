//! The tests connect as each API's login role, not as the superuser (MAIR-556).
//!
//! A superuser ignores the `GRANT`s of `repeatable/security/api_grants.sql` (Database), so a
//! query an API's role may not run used to pass in CI and fail in staging with
//! `permission denied`. These tests check that the roles get a password from the test setup,
//! that the queries this crate runs on every request of every API are granted to every role,
//! and that a query outside a role's scope is refused.

use mairie360_api_lib::database::db_interface::Database;
use mairie360_api_lib::database::query_views::{
    GetUserIdByEmailQueryView, HasAccessQueryView, IsAdminQueryView, IsUserActiveByIdQueryView,
};
use mairie360_api_lib::test_setup::db_setup::API_ROLES;
use mairie360_api_lib::test_setup::http_setup::{test_app_state_as, SeededUser};
use mairie360_api_lib::test_setup::queries_setup::{get_shared_db, get_shared_db_as};
use serial_test::serial;
use sqlx::error::DatabaseError;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

/// SQLSTATE of `permission denied`.
const INSUFFICIENT_PRIVILEGE: &str = "42501";

async fn raw_pool(url: &str) -> PgPool {
    PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(std::time::Duration::from_secs(3))
        .connect(url)
        .await
        .expect("Failed to connect to the test database")
}

#[tokio::test]
#[serial]
async fn every_api_role_connects_as_itself_without_superuser_rights() {
    for role in API_ROLES {
        let pool = raw_pool(&get_shared_db_as(role).await).await;
        let (user, is_superuser): (String, bool) = sqlx::query_as(
            "SELECT current_user::TEXT, rolsuper FROM pg_roles WHERE rolname = current_user",
        )
        .fetch_one(&pool)
        .await
        .unwrap_or_else(|e| panic!("{role} cannot connect: {e}"));

        assert_eq!(user, role);
        assert!(!is_superuser, "{role} must not be a superuser");
    }
}

#[tokio::test]
#[serial]
async fn the_shared_db_stays_superuser_for_the_fixtures() {
    let (_container, url) = get_shared_db().await;
    let pool = raw_pool(url).await;
    let is_superuser: bool =
        sqlx::query_scalar("SELECT rolsuper FROM pg_roles WHERE rolname = current_user")
            .fetch_one(&pool)
            .await
            .unwrap();

    assert!(is_superuser);
}

/// The middlewares of this crate run these views for every API: the JWT check
/// (`is_user_active_by_id`), the Keycloak account lookup (`get_user_id_by_email`), the admin
/// guard (`is_admin`) and the access guard (`check_access`).
#[tokio::test]
#[serial]
async fn every_api_role_runs_the_middleware_queries() {
    let alice = SeededUser::Alice.id().await;

    for role in API_ROLES {
        let db = Database::new(&get_shared_db_as(role).await).await;

        let active: bool = db
            .fetch_scalar(&IsUserActiveByIdQueryView::new(alice))
            .await
            .unwrap_or_else(|e| panic!("{role}: is_user_active_by_id failed: {e}"));
        assert!(active, "{role}: Alice is active");

        let id: Option<i32> = db
            .fetch_scalar(&GetUserIdByEmailQueryView::new("ALICE@example.com"))
            .await
            .unwrap_or_else(|e| panic!("{role}: get_user_id_by_email failed: {e}"));
        assert_eq!(
            id.map(i64::from),
            Some(i64::try_from(alice).unwrap()),
            "{role}: Keycloak e-mail lookup"
        );

        let _: bool = db
            .fetch_scalar(&IsAdminQueryView::new(alice))
            .await
            .unwrap_or_else(|e| panic!("{role}: is_admin failed: {e}"));

        let _: i32 = db
            .fetch_scalar(&HasAccessQueryView::new(alice, "document", "read", Some(1)))
            .await
            .unwrap_or_else(|e| panic!("{role}: check_access failed: {e}"));
    }
}

#[tokio::test]
#[serial]
async fn a_query_outside_the_role_scope_is_refused() {
    // project_api reads the users' names, never their password hashes nor their sessions.
    let pool = raw_pool(&get_shared_db_as("project_api").await).await;

    for sql in [
        "SELECT password FROM users LIMIT 1",
        "SELECT token_hash FROM sessions LIMIT 1",
    ] {
        let error = sqlx::query(sql)
            .fetch_optional(&pool)
            .await
            .expect_err("project_api must not run this query");
        let code = error.as_database_error().and_then(DatabaseError::code);

        assert_eq!(code.as_deref(), Some(INSUFFICIENT_PRIVILEGE), "{sql}");
    }

    // The same query passes as the superuser, which is why the APIs must not test with it.
    let (_container, url) = get_shared_db().await;
    let superuser = raw_pool(url).await;
    sqlx::query("SELECT password FROM users LIMIT 1")
        .fetch_optional(&superuser)
        .await
        .expect("the superuser ignores the grants");
}

#[tokio::test]
#[serial]
async fn the_app_state_of_an_api_role_is_ready() {
    let state = test_app_state_as("compliance_api").await;

    assert!(state.readiness().await.postgres);
}

#[tokio::test]
#[should_panic(expected = "is not an API role")]
async fn an_unknown_role_is_refused() {
    get_shared_db_as("postgres").await;
}
