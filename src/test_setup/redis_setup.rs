use redis::Client;
use std::env;
use testcontainers::core::{ContainerPort, WaitFor};
use testcontainers::runners::AsyncRunner;
use testcontainers::ContainerAsync;
use testcontainers::GenericImage;

pub struct RedisTestConfig {
    pub url: String,
    pub host: String,
    pub port: u16,
}

/// Démarre un conteneur Redis et attend qu'il soit prêt
pub async fn start_redis_container() -> (ContainerAsync<GenericImage>, RedisTestConfig) {
    let node = GenericImage::new("redis", "7.2.4")
        .with_exposed_port(ContainerPort::Tcp(6379))
        .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
        .start()
        .await
        .expect("Failed to start Redis");

    let host = node.get_host().await.unwrap().to_string();
    let port = node.get_host_port_ipv4(6379).await.unwrap();
    let url = format!("redis://{}:{}", host, port);

    (node, RedisTestConfig { url, host, port })
}

/// Configure la variable d'environnement pour le RedisManager de la lib
pub fn set_redis_env_var(config: &RedisTestConfig) {
    env::set_var("REDIS_URL", &config.url);
}

/// Helper pour obtenir une connexion directe (pour les tests de fonctions simples)
pub async fn get_redis_connection(config: &RedisTestConfig) -> redis::Connection {
    let client = Client::open(config.url.as_str()).expect("Invalid Redis URL");
    client.get_connection().expect("Failed to connect to Redis")
}
