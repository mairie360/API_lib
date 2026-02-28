use std::env;
use std::time::Duration;
use testcontainers::GenericImage;
use testcontainers::runners::AsyncRunner;
use testcontainers::ImageExt;
use testcontainers::core::ContainerPort;
use testcontainers::ContainerAsync;
use tokio_postgres::NoTls;

pub struct TestDbConfig {
    pub host: String,
    pub port: u16,
}

/// Démarre un conteneur Postgres standard
pub async fn start_postgres_container() -> (ContainerAsync<GenericImage>, TestDbConfig) {
    let node = GenericImage::new("postgres", "15-alpine")
        .with_exposed_port(ContainerPort::Tcp(5432))
        .with_env_var("POSTGRES_USER", "postgres")
        .with_env_var("POSTGRES_PASSWORD", "postgres")
        .with_env_var("POSTGRES_DB", "postgres")
        .start()
        .await
        .expect("Failed to start Postgres");

    let host = node.get_host().await.unwrap().to_string();
    let port = node.get_host_port_ipv4(5432).await.unwrap();
    let config = TestDbConfig { host, port };

    // --- HEALTHCHECK MANUEL ---
    // On essaie de se connecter via tokio_postgres directement jusqu'à ce que ça réponde
    let connection_string = format!(
        "host={} port={} user=postgres password=postgres dbname=postgres",
        config.host, config.port
    );

    let mut attempts = 0;
    while attempts < 15 {
        match tokio_postgres::connect(&connection_string, NoTls).await {
            Ok(_) => break,
            Err(_) => {
                tokio::time::sleep(Duration::from_millis(300)).await;
                attempts += 1;
            }
        }
    }
    // --------------------------

    (node, config)
}
/// Configure les variables d'environnement pour la lib
pub fn set_db_env_vars(config: &TestDbConfig, db_name: &str, user: &str, pass: &str) {
    env::set_var("DB_HOST", &config.host);
    env::set_var("DB_PORT", config.port.to_string());
    env::set_var("DB_NAME", db_name);
    env::set_var("DB_USER", user);
    env::set_var("DB_PASSWORD", pass);
}
