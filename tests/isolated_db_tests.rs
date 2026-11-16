//! One database and one Redis key space per test (MAIR-1002).
//!
//! None of these tests is `#[serial]`: that is what `isolated_db()` / `isolated_redis()` are for.
//! They write rows the other tests of the binary must never see.

use mairie360_api_lib::database::db_interface::Database;
use mairie360_api_lib::database::query_views::{IsAdminQueryView, IsUserActiveByIdQueryView};
use mairie360_api_lib::jwt_manager::{is_session_revoked, revoke_session};
use mairie360_api_lib::test_setup::http_setup::SeededUser;
use mairie360_api_lib::test_setup::isolated::{isolated_db, IsolatedDb};
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use mairie360_api_lib::test_setup::redis_setup::{get_shared_redis, isolated_redis};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

async fn raw_pool(url: &str) -> PgPool {
    PgPoolOptions::new()
        .max_connections(2)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(url)
        .await
        .expect("Failed to connect to the test database")
}

async fn count_users(pool: &PgPool, email: &str) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn insert_user(pool: &PgPool, email: &str) {
    sqlx::query(
        "INSERT INTO users (first_name, last_name, email, password, status) \
         VALUES ('Iso', 'Lated', $1, $2, 'active')",
    )
    .bind(email)
    .bind(mairie360_api_lib::test_setup::queries_setup::seed_password_hash())
    .execute(pool)
    .await
    .unwrap();
}

async fn database_exists(name: &str) -> bool {
    let (_container, shared_url) = get_shared_db().await;
    let pool = raw_pool(shared_url).await;
    sqlx::query_scalar::<_, bool>("SELECT EXISTS (SELECT 1 FROM pg_database WHERE datname = $1)")
        .bind(name)
        .fetch_one(&pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn an_isolated_database_starts_from_the_seeded_data() {
    let db = isolated_db().await;
    let pool = raw_pool(&db.url()).await;

    let alice = SeededUser::Alice.id().await;
    let emails: Vec<String> = sqlx::query_scalar("SELECT email FROM users ORDER BY id")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(
        emails,
        [
            "alice@example.com",
            "bob@example.com",
            "admin@test.com",
            "owner@test.com"
        ],
        "the copy holds exactly the seed (ids are the SeededUser ones, alice is {alice})"
    );
}

#[tokio::test]
async fn what_a_test_writes_in_its_database_reaches_no_other_database() {
    let first = isolated_db().await;
    let second = isolated_db().await;
    let (_container, shared_url) = get_shared_db().await;
    let email = format!("only-in-{}@example.com", first.name());

    insert_user(&raw_pool(&first.url()).await, &email).await;

    assert_eq!(count_users(&raw_pool(&first.url()).await, &email).await, 1);
    assert_eq!(count_users(&raw_pool(&second.url()).await, &email).await, 0);
    assert_eq!(count_users(&raw_pool(shared_url).await, &email).await, 0);
}

#[tokio::test]
async fn the_seed_template_is_not_changed_by_the_writes_of_a_copy() {
    let writer = isolated_db().await;
    let writer_pool = raw_pool(&writer.url()).await;
    insert_user(&writer_pool, "template-must-not-see@example.com").await;
    sqlx::query("UPDATE users SET first_name = 'Changed' WHERE email = 'alice@example.com'")
        .execute(&writer_pool)
        .await
        .unwrap();

    let reader = isolated_db().await;
    let pool = raw_pool(&reader.url()).await;
    let first_name: String =
        sqlx::query_scalar("SELECT first_name FROM users WHERE email = 'alice@example.com'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(first_name, "Alice");
    assert_eq!(
        count_users(&pool, "template-must-not-see@example.com").await,
        0
    );
}

#[tokio::test]
async fn the_api_roles_work_on_a_copy_under_their_grants() {
    let db = isolated_db().await;
    let alice = SeededUser::Alice.id().await;
    let bob = SeededUser::Bob.id().await;

    for role in ["core_api", "project_api"] {
        let database = Database::new(&db.url_as(role)).await;
        assert!(
            database
                .fetch_scalar::<bool, _>(&IsAdminQueryView::new(alice))
                .await
                .unwrap(),
            "{role}: alice is an admin in the copy"
        );
        assert!(database
            .fetch_scalar::<bool, _>(&IsUserActiveByIdQueryView::new(alice))
            .await
            .unwrap());
        assert!(
            !database
                .fetch_scalar::<bool, _>(&IsUserActiveByIdQueryView::new(bob))
                .await
                .unwrap(),
            "{role}: bob is archived in the copy"
        );
    }
}

#[tokio::test]
#[should_panic(expected = "is not an API role")]
async fn an_unknown_role_is_refused() {
    let db = isolated_db().await;
    let _ = db.url_as("not_a_role");
}

#[tokio::test]
async fn a_dropped_database_is_removed_even_with_its_pool_still_open() {
    let db = isolated_db().await;
    let name = db.name().to_owned();
    let pool = raw_pool(&db.url()).await;
    assert!(database_exists(&name).await);

    drop(db);

    assert!(!database_exists(&name).await, "{name} must be dropped");
    assert!(sqlx::query("SELECT 1").execute(&pool).await.is_err());
}

#[tokio::test]
async fn databases_can_be_created_in_parallel_with_distinct_names() {
    let databases = futures_util::future::join_all((0..6).map(|_| isolated_db())).await;
    let mut names: Vec<&str> = databases.iter().map(IsolatedDb::name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), 6);
    for db in &databases {
        assert_eq!(
            count_users(&raw_pool(&db.url()).await, "alice@example.com").await,
            1
        );
    }
}

#[tokio::test]
async fn two_isolated_redis_clients_do_not_see_each_others_keys() {
    let first = isolated_redis().await;
    let second = isolated_redis().await;
    assert_ne!(first.key_prefix(), second.key_prefix());

    first.set_ex("session", "one", 60).await.unwrap();

    assert_eq!(
        first
            .get::<Option<String>>("session")
            .await
            .unwrap()
            .as_deref(),
        Some("one")
    );
    assert_eq!(second.get::<Option<String>>("session").await.unwrap(), None);
    second.set_ex("session", "two", 60).await.unwrap();
    assert_eq!(
        first
            .get::<Option<String>>("session")
            .await
            .unwrap()
            .as_deref(),
        Some("one")
    );
}

// CodeRabbit on MAIR-1002: the revocation keys bypass the client prefix in production (every API
// reads the same `revoked:<sid>`), so the isolated clients namespace them too.
#[tokio::test]
async fn revoking_a_session_through_an_isolated_client_does_not_revoke_it_for_another() {
    let first = isolated_redis().await;
    let second = isolated_redis().await;

    revoke_session(&first, "session-1", 60).await.unwrap();

    assert!(is_session_revoked(&first, "session-1").await.unwrap());
    assert!(!is_session_revoked(&second, "session-1").await.unwrap());
}

#[tokio::test]
async fn the_shared_redis_is_one_container_for_the_whole_process() {
    let first = get_shared_redis().await;
    let second = get_shared_redis().await;
    assert_eq!(first.port, second.port);
}
