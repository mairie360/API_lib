//! One database per test (MAIR-1002).
//!
//! `get_shared_db()` gives every test of a binary the same seeded database, so a test that writes
//! rows can only run alone (`#[serial]`). [`isolated_db`] gives a test a database of its own: a copy
//! of the seeded one, made by Postgres with `CREATE DATABASE … TEMPLATE` from a template taken once
//! per process, right after the seed and before any test touched the data. Same schema, grants and
//! seeded rows (so the same `ALICE_ID`, `SeededUser`…), nothing mocked; the copy is dropped with the
//! returned [`IsolatedDb`].
//!
//! ```ignore
//! let db = isolated_db().await;
//! let pool = Database::new(&db.url_as("core_api")).await;
//! // … any write: no other test sees it, no `#[serial]` needed
//! ```

use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use tokio_postgres::error::SqlState;
use tokio_postgres::NoTls;

use super::queries_setup::{get_shared_db, role_url, SEED_TEMPLATE};

/// Attempts of a `CREATE DATABASE` refused because the template is still in use (the session of the
/// seed fixtures closing), spaced by [`RETRY_DELAY`].
const CREATE_ATTEMPTS: u32 = 100;

const RETRY_DELAY: Duration = Duration::from_millis(100);

/// Upper bound of the drop of a test database, so an unresponsive server cannot block a test. Long on
/// purpose: a loaded Postgres (the whole suite at once) can take several seconds to end the sessions.
const CLEANUP_TIMEOUT: Duration = Duration::from_secs(60);

/// Numbers the databases of this process (the container is per process, so it is unique enough).
static COUNTER: AtomicU32 = AtomicU32::new(0);

/// `postgres://user:password@host:port` of `database_url` (`…/<db>` removed).
fn server_url(database_url: &str) -> &str {
    database_url
        .rsplit_once('/')
        .expect("the shared test database URL ends with its database name")
        .0
}

/// `CREATE DATABASE "name" TEMPLATE "template"`, retried while the template has other sessions.
///
/// # Panics
///
/// Panics when the database cannot be created: a test cannot go on without it.
pub(super) async fn create_database(server_url: &str, name: &str, template: &str) {
    let (client, connection) = tokio_postgres::connect(&format!("{server_url}/template1"), NoTls)
        .await
        .expect("Failed to connect to Postgres to create a test database");
    tokio::spawn(connection);
    let sql = format!("CREATE DATABASE \"{name}\" TEMPLATE \"{template}\"");
    for attempt in 1..=CREATE_ATTEMPTS {
        match client.batch_execute(&sql).await {
            Ok(()) => return,
            Err(error)
                if error.code() == Some(&SqlState::OBJECT_IN_USE) && attempt < CREATE_ATTEMPTS =>
            {
                tokio::time::sleep(RETRY_DELAY).await;
            }
            Err(error) => panic!("Failed to create the test database {name}: {error}"),
        }
    }
}

/// A database of its own for one test, a copy of the seeded one. Dropped (connections closed, files
/// removed) when this value is.
#[derive(Debug)]
pub struct IsolatedDb {
    name: String,
    server_url: String,
}

impl IsolatedDb {
    /// Name of the database on the shared Postgres.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// URL of the database as the superuser: for the fixtures only (a superuser ignores the `GRANT`s).
    #[must_use]
    pub fn url(&self) -> String {
        format!("{}/{}", self.server_url, self.name)
    }

    /// URL of the database as the login role of an API (`core_api`, `project_api`… see
    /// `db_setup::API_ROLES`): connect the code under test with it, like staging does.
    ///
    /// # Panics
    ///
    /// Panics if `role` is not one of `db_setup::API_ROLES`.
    #[must_use]
    pub fn url_as(&self, role: &str) -> String {
        assert!(
            super::db_setup::API_ROLES.contains(&role),
            "{role} is not an API role of the test database, expected one of {:?}",
            super::db_setup::API_ROLES
        );
        role_url(&self.url(), role)
    }
}

impl Drop for IsolatedDb {
    fn drop(&mut self) {
        let server_url = self.server_url.clone();
        let name = self.name.clone();
        // A thread of its own: `Drop` may run inside the test's runtime, which cannot block on a
        // future, and the pools of the test may still be open (`WITH (FORCE)` closes them).
        let _ = std::thread::spawn(move || {
            let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            else {
                return;
            };
            runtime.block_on(async move {
                let cleanup = async {
                    let Ok((client, connection)) =
                        tokio_postgres::connect(&format!("{server_url}/template1"), NoTls).await
                    else {
                        return;
                    };
                    tokio::spawn(connection);
                    let _ = client
                        .batch_execute(&format!("DROP DATABASE IF EXISTS \"{name}\" WITH (FORCE)"))
                        .await;
                };
                if tokio::time::timeout(CLEANUP_TIMEOUT, cleanup)
                    .await
                    .is_err()
                {
                    eprintln!("timed out dropping test database {name}");
                }
            });
        })
        .join();
    }
}

/// A new database for the calling test: a copy of the seeded database (see the module).
///
/// Starts the shared container and seeds it on the first call of the process, like
/// [`get_shared_db`].
///
/// # Panics
///
/// Panics if the shared database cannot be set up or the copy cannot be made.
pub async fn isolated_db() -> IsolatedDb {
    let (_container, shared_url) = get_shared_db().await;
    let server_url = server_url(shared_url).to_owned();
    let name = format!(
        "isolated_{}_{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    );
    create_database(&server_url, &name, SEED_TEMPLATE).await;
    IsolatedDb { name, server_url }
}
