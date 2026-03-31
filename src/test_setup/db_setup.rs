use std::env;
use std::time::Duration;
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, GenericImage, ImageExt};
use tokio_postgres::NoTls;

static DB_VERSION: &str = "dev-b816587";

pub struct TestDbConfig {
    pub host: String,
    pub port: u16,
}

/// Lance les migrations Liquibase via ton image personnalisée
pub async fn run_migrations(_container: &ContainerAsync<GenericImage>) {
    let liquibase_url = "jdbc:postgresql://127.0.0.1:5432/postgres";

    println!("🚀 Liquibase connectant à : {}", liquibase_url);

    let liquibase_node = GenericImage::new("ghcr.io/mairie360/liquibase-migrations", DB_VERSION)
        .with_network("host")
        .with_working_dir("/migrations") // Correspond au WORKDIR de ton Dockerfile
        .with_env_var("LIQUIBASE_SEARCH_PATH", "/migrations") // Comme dans ton Compose
        .with_cmd(vec![
            "update",
            "--url",
            liquibase_url,
            "--username",
            "postgres",
            "--password",
            "postgres",
            "--changelog-file",
            "changelog.xml", // Relatif à /migrations
        ])
        .start()
        .await
        .expect("Failed to start Liquibase container");

    while liquibase_node.is_running().await.unwrap() {
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    // Logs pour debug
    let stdout = liquibase_node.stdout_to_vec().await.unwrap_or_default();
    let stderr = liquibase_node.stderr_to_vec().await.unwrap_or_default();
    println!("STDOUT: {}", String::from_utf8_lossy(&stdout));
    eprintln!("STDERR: {}", String::from_utf8_lossy(&stderr));

    println!("✅ Fin du container Liquibase.");
}

/// Démarre un conteneur Postgres standard
pub async fn start_postgres_container() -> (ContainerAsync<GenericImage>, TestDbConfig) {
    let node = GenericImage::new("ghcr.io/mairie360/database", DB_VERSION)
        .with_network("host") // Mode host pour la simplicité sous Linux
        .with_env_var("POSTGRES_USER", "postgres")
        .with_env_var("POSTGRES_PASSWORD", "postgres")
        .with_env_var("POSTGRES_DB", "postgres")
        .start()
        .await
        .expect("Failed to start Postgres");

    // En mode host, on tape directement sur 127.0.0.1:5432
    let config = TestDbConfig {
        host: "127.0.0.1".to_string(),
        port: 5432,
    };

    let connection_string =
        "host=127.0.0.1 port=5432 user=postgres password=postgres dbname=postgres";

    // Healthcheck
    let mut attempts = 0;
    while attempts < 15 {
        if tokio_postgres::connect(connection_string, NoTls)
            .await
            .is_ok()
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
        attempts += 1;
    }

    run_migrations(&node).await;
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
