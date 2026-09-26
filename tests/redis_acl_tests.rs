//! MAIR-267: keys are prefixed with the API role, as the Redis ACL of the platform requires.
//! Runs against a Redis started with the chart's exact ACL lines (`chart_acl_lines`).
//!
//! Own test binary: some tests change `REDIS_KEY_PREFIX` / `REDIS_USERNAME`.

use mairie360_api_lib::jwt_manager::{is_session_revoked, revoke_session};
use mairie360_api_lib::redis::redis_interface::{resolve_key_prefix, Redis};
use mairie360_api_lib::test_setup::redis_setup::{start_acl_redis_container, ACL_TEST_PASSWORD};
use redis::Commands;
use serial_test::serial;

/// Raw connection as `admin`, to look at the keys really written.
fn admin(
    config: &mairie360_api_lib::test_setup::redis_setup::RedisTestConfig,
) -> redis::Connection {
    redis::Client::open(config.url_as("admin"))
        .unwrap()
        .get_connection()
        .unwrap()
}

#[tokio::test]
#[serial]
async fn role_prefix_comes_from_the_url_and_keys_work_under_the_acl() {
    std::env::remove_var("REDIS_KEY_PREFIX");
    std::env::remove_var("REDIS_USERNAME");
    let (_node, config) = start_acl_redis_container().await;
    let core = Redis::new(&config.url_as("core-api"));
    assert_eq!(core.key_prefix(), Some("core-api"));

    core.set("plain", "value").await.unwrap();
    assert_eq!(core.get::<String>("plain").await.unwrap(), "value");
    core.set_ex("temporary", "value", 120).await.unwrap();
    assert!(core.secure_set_ex("once", "first", 60).await.unwrap());
    assert!(!core.secure_set_ex("once", "second", 60).await.unwrap());
    assert_eq!(
        core.secure_get::<String>("once").await.unwrap().as_deref(),
        Some("first")
    );
    core.secure_set("cache", "value").await.unwrap();
    core.expire("cache", 30).await.unwrap();
    assert!(core.key_exist("cache").await.unwrap());
    core.secure_delete("plain").await.unwrap();

    let mut admin = admin(&config);
    let plain: bool = admin.exists("core-api:plain").unwrap();
    assert!(!plain, "deleted");
    let ttl: i64 = admin.ttl("core-api:temporary").unwrap();
    assert!((1..=120).contains(&ttl), "ttl = {ttl}");
    let ttl: i64 = admin.ttl("core-api:once").unwrap();
    assert!((1..=60).contains(&ttl), "ttl = {ttl}");
    let ttl: i64 = admin.ttl("core-api:cache").unwrap();
    assert!((1..=30).contains(&ttl), "ttl = {ttl}");
    let unprefixed: bool = admin.exists("temporary").unwrap();
    assert!(!unprefixed);
}

#[tokio::test]
#[serial]
async fn unprefixed_or_foreign_keys_are_refused() {
    let (_node, config) = start_acl_redis_container().await;

    // Without the prefix, the ACL refuses the key: this was the bug.
    let unprefixed = Redis::with_key_prefix(&config.url_as("core-api"), None);
    assert!(unprefixed.set("plain", "value").await.is_err());

    // A role cannot touch another role's keys.
    let project_as_core = Redis::with_key_prefix(&config.url_as("project-api"), Some("core-api"));
    assert!(project_as_core.set("plain", "value").await.is_err());
    assert!(project_as_core.key_exist("plain").await.is_err());

    // Its own keys work.
    let project = Redis::new(&config.url_as("project-api"));
    assert_eq!(project.key_prefix(), Some("project-api"));
    project.set_ex("plain", "value", 60).await.unwrap();
}

#[tokio::test]
#[serial]
async fn revocation_keys_are_shared_and_unprefixed() {
    let (_node, config) = start_acl_redis_container().await;
    let core = Redis::new(&config.url_as("core-api"));
    let project = Redis::new(&config.url_as("project-api"));
    let sid = uuid::Uuid::new_v4().to_string();

    // core-api writes, every API reads the same key.
    revoke_session(&core, &sid, 60).await.unwrap();
    assert!(is_session_revoked(&core, &sid).await.unwrap());
    assert!(is_session_revoked(&project, &sid).await.unwrap());

    let mut admin = admin(&config);
    let ttl: i64 = admin.ttl(format!("revoked:{sid}")).unwrap();
    assert!((1..=60).contains(&ttl), "ttl = {ttl}");

    // Other APIs can only read the list.
    let other = uuid::Uuid::new_v4().to_string();
    assert!(revoke_session(&project, &other, 60).await.is_err());
    assert!(!is_session_revoked(&core, &other).await.unwrap());
}

#[tokio::test]
#[serial]
async fn prefix_resolution_order() {
    let with_user = format!("redis://core-api:{ACL_TEST_PASSWORD}@redis:6379");

    std::env::remove_var("REDIS_KEY_PREFIX");
    std::env::remove_var("REDIS_USERNAME");
    assert_eq!(resolve_key_prefix("redis://redis:6379"), None);
    assert_eq!(resolve_key_prefix(&with_user).as_deref(), Some("core-api"));

    std::env::set_var("REDIS_USERNAME", "calendar-api");
    assert_eq!(
        resolve_key_prefix("redis://redis:6379").as_deref(),
        Some("calendar-api")
    );
    assert_eq!(resolve_key_prefix(&with_user).as_deref(), Some("core-api"));

    std::env::set_var("REDIS_KEY_PREFIX", "explicit");
    assert_eq!(resolve_key_prefix(&with_user).as_deref(), Some("explicit"));

    std::env::set_var("REDIS_KEY_PREFIX", "");
    assert_eq!(resolve_key_prefix(&with_user).as_deref(), Some("core-api"));

    std::env::remove_var("REDIS_KEY_PREFIX");
    std::env::remove_var("REDIS_USERNAME");

    // No prefix: keys are sent as they are (local Redis without ACL).
    let local = Redis::with_key_prefix("redis://redis:6379", None);
    assert_eq!(local.full_key("a/b"), "a/b");
    let core = Redis::with_key_prefix("redis://redis:6379", Some("core-api"));
    assert_eq!(core.full_key("a/b"), "core-api:a/b");
}
