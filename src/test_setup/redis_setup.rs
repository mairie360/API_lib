use redis::Client;
use std::env;
use testcontainers::core::{ContainerPort, WaitFor};
use testcontainers::runners::AsyncRunner;
use testcontainers::ContainerAsync;
use testcontainers::GenericImage;
use testcontainers::ImageExt;

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
            "core-api on >{ACL_TEST_PASSWORD} ~core-api:* ~revoked:* resetchannels -@all {commands}"
        ),
        format!(
            "project-api on >{ACL_TEST_PASSWORD} ~project-api:* %R~revoked:* resetchannels -@all {commands}"
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
