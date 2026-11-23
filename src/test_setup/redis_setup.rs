use super::cleanup::remove_at_exit;
use crate::redis::redis_interface::Redis;
use futures_util::FutureExt;
use redis::Client;
use std::env;
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicU32, Ordering};
use testcontainers::core::{ContainerPort, WaitFor};
use testcontainers::runners::AsyncRunner;
use testcontainers::ContainerAsync;
use testcontainers::GenericImage;
use testcontainers::ImageExt;
use tokio::sync::OnceCell;

pub struct RedisTestConfig {
    pub url: String,
    pub host: String,
    pub port: u16,
}

/// Démarre un conteneur Redis et attend qu'il soit prêt
///
/// # Panics
///
/// Panique si le conteneur ou la base de test ne peut pas être préparé : un test ne peut pas
/// continuer sans son environnement.
pub async fn start_redis_container() -> (ContainerAsync<GenericImage>, RedisTestConfig) {
    let node = GenericImage::new("redis", "7.2.4")
        .with_exposed_port(ContainerPort::Tcp(6379))
        .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
        .start()
        .await
        .expect("Failed to start Redis");

    let host = node.get_host().await.unwrap().to_string();
    let port = node.get_host_port_ipv4(6379).await.unwrap();
    let url = format!("redis://{host}:{port}");

    (node, RedisTestConfig { url, host, port })
}

/// Configure la variable d'environnement pour le `RedisManager` de la lib
pub fn set_redis_env_var(config: &RedisTestConfig) {
    env::set_var("REDIS_URL", &config.url);
}

/// Helper pour obtenir une connexion directe (pour les tests de fonctions simples)
///
/// # Panics
///
/// Panique si le conteneur ou la base de test ne peut pas être préparé : un test ne peut pas
/// continuer sans son environnement.
#[must_use]
pub fn get_redis_connection(config: &RedisTestConfig) -> redis::Connection {
    let client = Client::open(config.url.as_str()).expect("Invalid Redis URL");
    client.get_connection().expect("Failed to connect to Redis")
}

/// Password of every ACL user of [`start_acl_redis_container`].
pub const ACL_TEST_PASSWORD: &str = "acl-test-password";

/// ACL users of [`start_acl_redis_container`], exactly as the Deploiment chart writes them.
///
/// Same lines as the chart's `entrypoint.sh` (MAIR-264 / MAIR-267): `default` disabled, an `admin` user, the
/// `core-api` role (read-write on `revoked:*`) and the `project-api` role (read-only on
/// `revoked:*`). Keep in sync with `charts/mairie360-stack/charts/redis`.
#[must_use]
pub fn chart_acl_lines() -> Vec<String> {
    let commands = "+get +set +del +exists +expire";
    vec![
        "default off resetkeys resetchannels -@all".to_string(),
        format!("admin on >{ACL_TEST_PASSWORD} ~* &* allcommands"),
        format!(
            "core-api on >{ACL_TEST_PASSWORD} ~core-api:* ~revoked:* ~session:* resetchannels -@all {commands}"
        ),
        format!(
            "project-api on >{ACL_TEST_PASSWORD} ~project-api:* %R~revoked:* ~session:* resetchannels -@all {commands}"
        ),
    ]
}

/// Starts a Redis 7.4 (the chart's image) enforcing [`chart_acl_lines`]. Connect with
/// `redis://<user>:ACL_TEST_PASSWORD@host:port`; [`RedisTestConfig::url`] has no credentials.
///
/// # Panics
///
/// Panics when the container cannot be started.
pub async fn start_acl_redis_container() -> (ContainerAsync<GenericImage>, RedisTestConfig) {
    let mut cmd = vec!["redis-server".to_string()];
    for line in chart_acl_lines() {
        cmd.push("--user".to_string());
        cmd.extend(line.split(' ').map(str::to_string));
    }
    let node = GenericImage::new("redis", "7.4-alpine")
        .with_exposed_port(ContainerPort::Tcp(6379))
        .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
        .with_cmd(cmd)
        .start()
        .await
        .expect("Failed to start Redis with ACL");

    let host = node.get_host().await.unwrap().to_string();
    let port = node.get_host_port_ipv4(6379).await.unwrap();
    let url = format!("redis://{host}:{port}");

    (node, RedisTestConfig { url, host, port })
}

impl RedisTestConfig {
    /// URL of this Redis authenticated as `user` (see [`chart_acl_lines`]).
    #[must_use]
    pub fn url_as(&self, user: &str) -> String {
        format!(
            "redis://{user}:{ACL_TEST_PASSWORD}@{}:{}",
            self.host, self.port
        )
    }
}

/// Outcome of the one-time start of the shared Redis, failure included (same reason as the shared
/// database: a `OnceCell` whose initialiser panics stays empty).
static SHARED_REDIS: OnceCell<Result<(ContainerAsync<GenericImage>, RedisTestConfig), String>> =
    OnceCell::const_new();

/// Numbers the key prefixes of this process.
static PREFIX_COUNTER: AtomicU32 = AtomicU32::new(0);

/// The Redis shared by the tests of the process, started on the first call and removed at exit.
///
/// One container instead of one per test: isolate the keys of a test with [`isolated_redis`] (or
/// [`unique_redis_prefix`]), not with a container of its own.
///
/// # Panics
///
/// Panics if the container cannot be started, and on every later call too (nothing is retried).
pub async fn get_shared_redis() -> &'static RedisTestConfig {
    let outcome = SHARED_REDIS
        .get_or_init(|| async {
            AssertUnwindSafe(async {
                let (node, config) = start_redis_container().await;
                remove_at_exit(node.id());
                (node, config)
            })
            .catch_unwind()
            .await
            .map_err(|payload| {
                payload
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| payload.downcast_ref::<&str>().map(ToString::to_string))
                    .unwrap_or_else(|| "unknown panic".to_owned())
            })
        })
        .await;
    match outcome {
        Ok((_container, config)) => config,
        Err(reason) => {
            panic!("The shared Redis could not be started, nothing was retried: {reason}")
        }
    }
}

/// A key prefix no other call of this process returns (`t<pid>x<n>`, without the `:` separator
/// [`Redis`] adds).
#[must_use]
pub fn unique_redis_prefix() -> String {
    format!(
        "t{}x{}",
        std::process::id(),
        PREFIX_COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

/// A [`Redis`] client of the shared Redis whose keys all carry a prefix of its own.
///
/// Two tests never read each other's keys, without a container per test. The shared revocation keys
/// (`revoke_session`, `is_session_revoked`) are isolated too: they live under the same prefix.
///
/// # Panics
///
/// Panics if the shared Redis cannot be started.
pub async fn isolated_redis() -> Redis {
    let config = get_shared_redis().await;
    Redis::with_isolated_shared_keys(&config.url, &unique_redis_prefix())
}
