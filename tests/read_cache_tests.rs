//! MAIR-524: read-through cache of the heavy list routes (`ReadCache`, `SmartDatabase::cached`).
//! Cache hits, isolation between users, invalidation after a write (direct, `fetch_*` with
//! `RETURNING`, transaction committed or rolled back), Redis down, and the chart's Redis ACL.

use std::sync::atomic::{AtomicUsize, Ordering};

use mairie360_api_lib::database::db_interface::{ApiRequestDto, Database, QueryParam};
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::redis::redis_interface::Redis;
use mairie360_api_lib::smart_db::{generation_key, ReadCache, SmartDatabase};
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use mairie360_api_lib::test_setup::redis_setup::{
    start_acl_redis_container, start_redis_container,
};
use serde::{Deserialize, Serialize};
use serial_test::serial;

const ROWS_CACHE: ReadCache = ReadCache::new("mair524-rows", 60);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Page {
    owner: u64,
    rows: Vec<i32>,
}

/// `SELECT` of a test table's rows, through the cache of [`ROWS_CACHE`].
#[derive(Debug, Deserialize)]
struct ListRowsView {
    params: Vec<QueryParam>,
}

impl ApiRequestDto for ListRowsView {
    fn query_sql(&self) -> &'static str {
        "SELECT COALESCE(jsonb_agg(id ORDER BY id), '[]'::jsonb) FROM mair524_rows WHERE owner = $1"
    }
    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

/// A write of the table, declaring the namespace it makes stale.
#[derive(Debug, Deserialize)]
struct InsertRowView {
    params: Vec<QueryParam>,
}

impl InsertRowView {
    fn new(owner: i32) -> Self {
        Self {
            params: vec![QueryParam::I32(owner)],
        }
    }
}

impl ApiRequestDto for InsertRowView {
    fn query_sql(&self) -> &'static str {
        "INSERT INTO mair524_rows (owner) VALUES ($1) RETURNING id"
    }
    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
    fn invalidated_read_caches(&self) -> &'static [&'static str] {
        &["mair524-rows"]
    }
}

/// Same write, declaring no namespace.
#[derive(Debug, Deserialize)]
struct SilentInsertRowView {
    params: Vec<QueryParam>,
}

impl ApiRequestDto for SilentInsertRowView {
    fn query_sql(&self) -> &'static str {
        "INSERT INTO mair524_rows (owner) VALUES ($1)"
    }
    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

#[derive(Debug, Deserialize)]
struct CreateTableView {
    params: Vec<QueryParam>,
}

impl ApiRequestDto for CreateTableView {
    fn query_sql(&self) -> &'static str {
        "CREATE TABLE IF NOT EXISTS mair524_rows (id SERIAL PRIMARY KEY, owner INT NOT NULL)"
    }
    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

async fn smart_db(redis: Redis) -> SmartDatabase {
    let (_container, db_host) = get_shared_db().await;
    let db = Database::new(db_host.as_str()).await;
    db.execute(&CreateTableView { params: vec![] })
        .await
        .unwrap();
    SmartDatabase::new(db, redis)
}

/// Owner ids distinct per test, so tests sharing the table do not see each other's rows.
const fn owner(test: i32) -> i32 {
    524_000 + test
}

/// The page of `owner` read through the cache as `user`, counting the database loads.
async fn read(db: &SmartDatabase, user: u64, owner: i32, loads: &AtomicUsize) -> Page {
    let key = ROWS_CACHE.for_user(user, &owner);
    db.cached(&key, async || {
        loads.fetch_add(1, Ordering::SeqCst);
        let rows: Vec<i32> = db
            .get_db()
            .fetch_one(&ListRowsView {
                params: vec![QueryParam::I32(owner)],
            })
            .await?;
        Ok::<_, ApiLibError>(Page { owner: user, rows })
    })
    .await
    .unwrap()
}

#[tokio::test]
#[serial]
async fn second_read_is_a_cache_hit() {
    let (_node, config) = start_redis_container().await;
    let db = smart_db(Redis::new(&config.url)).await;
    let loads = AtomicUsize::new(0);
    db.execute(InsertRowView::new(owner(1))).await.unwrap();

    let first = read(&db, 1, owner(1), &loads).await;
    let second = read(&db, 1, owner(1), &loads).await;

    assert_eq!(first, second);
    assert_eq!(first.rows.len(), 1);
    assert_eq!(
        loads.load(Ordering::SeqCst),
        1,
        "the second read must not hit Postgres"
    );
}

#[tokio::test]
#[serial]
async fn users_never_share_an_entry() {
    let (_node, config) = start_redis_container().await;
    let db = smart_db(Redis::new(&config.url)).await;
    let loads = AtomicUsize::new(0);

    let alice = read(&db, 1, owner(2), &loads).await;
    let bob = read(&db, 2, owner(2), &loads).await;

    assert_eq!(alice.owner, 1);
    assert_eq!(bob.owner, 2, "Bob must not be served Alice's cached page");
    assert_eq!(loads.load(Ordering::SeqCst), 2);
}

#[tokio::test]
#[serial]
async fn a_write_invalidates_every_entry_of_the_namespace() {
    let (_node, config) = start_redis_container().await;
    let db = smart_db(Redis::new(&config.url)).await;
    let loads = AtomicUsize::new(0);

    assert_eq!(read(&db, 1, owner(3), &loads).await.rows, Vec::<i32>::new());
    assert_eq!(read(&db, 2, owner(3), &loads).await.rows, Vec::<i32>::new());

    // `fetch_scalar` of an `INSERT … RETURNING` invalidates too.
    let _: i32 = db
        .fetch_scalar(&InsertRowView::new(owner(3)))
        .await
        .unwrap();

    assert_eq!(read(&db, 1, owner(3), &loads).await.rows.len(), 1);
    assert_eq!(read(&db, 2, owner(3), &loads).await.rows.len(), 1);
    assert_eq!(loads.load(Ordering::SeqCst), 4);
}

#[tokio::test]
#[serial]
async fn a_write_without_namespace_keeps_the_cache() {
    let (_node, config) = start_redis_container().await;
    let db = smart_db(Redis::new(&config.url)).await;
    let loads = AtomicUsize::new(0);

    assert_eq!(read(&db, 1, owner(4), &loads).await.rows, Vec::<i32>::new());
    db.execute(SilentInsertRowView {
        params: vec![QueryParam::I32(owner(4))],
    })
    .await
    .unwrap();

    assert_eq!(
        read(&db, 1, owner(4), &loads).await.rows,
        Vec::<i32>::new(),
        "still cached"
    );
    db.invalidate_read_cache(ROWS_CACHE.namespace()).await;
    assert_eq!(read(&db, 1, owner(4), &loads).await.rows.len(), 1);
}

#[tokio::test]
#[serial]
async fn a_transaction_invalidates_on_commit_only() {
    let (_node, config) = start_redis_container().await;
    let db = smart_db(Redis::new(&config.url)).await;
    let loads = AtomicUsize::new(0);
    assert_eq!(read(&db, 1, owner(5), &loads).await.rows, Vec::<i32>::new());

    // Rolled back: nothing written, nothing invalidated.
    let failed = db
        .transaction(async |tx| {
            tx.execute(&InsertRowView::new(owner(5))).await?;
            Err::<(), _>(ApiLibError::Database(
                mairie360_api_lib::database::error::DbError::NotFound,
            ))
        })
        .await;
    assert!(failed.is_err());
    assert_eq!(read(&db, 1, owner(5), &loads).await.rows, Vec::<i32>::new());
    assert_eq!(loads.load(Ordering::SeqCst), 1);

    // Pending until the commit.
    let mut tx = db.begin().await.unwrap();
    let _: i32 = tx
        .fetch_scalar(&InsertRowView::new(owner(5)))
        .await
        .unwrap();
    assert_eq!(
        read(&db, 1, owner(5), &loads).await.rows,
        Vec::<i32>::new(),
        "not committed yet"
    );
    tx.commit().await.unwrap();

    assert_eq!(read(&db, 1, owner(5), &loads).await.rows.len(), 1);
}

#[tokio::test]
#[serial]
async fn entries_carry_the_ttl_and_the_generation_key_expires_too() {
    let (_node, config) = start_redis_container().await;
    let db = smart_db(Redis::new(&config.url)).await;
    let loads = AtomicUsize::new(0);
    db.invalidate_read_cache(ROWS_CACHE.namespace()).await;
    read(&db, 1, owner(6), &loads).await;

    let mut conn = mairie360_api_lib::test_setup::redis_setup::get_redis_connection(&config);
    let keys: Vec<String> = redis::cmd("KEYS")
        .arg("read-cache:mair524-rows:*")
        .query(&mut conn)
        .unwrap();
    assert!(keys.len() >= 2, "{keys:?}");
    for key in keys {
        let ttl: i64 = redis::cmd("TTL").arg(&key).query(&mut conn).unwrap();
        assert!(ttl > 0, "{key} has no TTL");
        if key == generation_key("mair524-rows") {
            assert!(ttl > 60);
        } else {
            assert!(ttl <= 60, "{key}: {ttl}");
        }
    }
}

#[tokio::test]
#[serial]
async fn redis_down_falls_back_to_postgres() {
    let db = smart_db(Redis::new("redis://127.0.0.1:1")).await;
    let loads = AtomicUsize::new(0);

    read(&db, 1, owner(7), &loads).await;
    read(&db, 1, owner(7), &loads).await;
    db.execute(InsertRowView::new(owner(7))).await.unwrap();

    assert_eq!(read(&db, 1, owner(7), &loads).await.rows.len(), 1);
    assert_eq!(loads.load(Ordering::SeqCst), 3);
}

#[tokio::test]
#[serial]
async fn works_under_the_chart_acl() {
    let (_node, config) = start_acl_redis_container().await;
    let db = smart_db(Redis::new(&config.url_as("project-api"))).await;
    let loads = AtomicUsize::new(0);

    read(&db, 1, owner(8), &loads).await;
    read(&db, 1, owner(8), &loads).await;
    assert_eq!(
        loads.load(Ordering::SeqCst),
        1,
        "cached under the project-api prefix"
    );

    db.execute(InsertRowView::new(owner(8))).await.unwrap();
    assert_eq!(read(&db, 1, owner(8), &loads).await.rows.len(), 1);
    assert_eq!(loads.load(Ordering::SeqCst), 2);
}
