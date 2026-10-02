use std::env;
use std::time::Duration;
use testcontainers::core::IntoContainerPort;
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, GenericImage, ImageExt};
use tokio_postgres::NoTls;

/// Version par défaut des images `ghcr.io/mairie360/database` et
/// `ghcr.io/mairie360/liquibase-migrations` utilisées pour les tests.
/// Surchargeable par les services consommateurs via la variable d'env `TEST_DB_VERSION`
/// (utile s'ils doivent tester contre une autre version que celle par défaut de la lib).
///
/// Pinned to the latest published `dev-<sha>` image: no release carries the fix of the admin seed
/// (MAIR-307) and the `user_identities` table (MAIR-141) yet.
const DEFAULT_DB_VERSION: &str = "dev-fb7c223";

/// Port interne du conteneur Postgres, publié sur un port hôte aléatoire.
const POSTGRES_PORT: u16 = 5432;

/// Nombre d'essais de connexion (espacés de 300 ms) avant de déclarer la base indisponible.
const HEALTHCHECK_ATTEMPTS: u32 = 100;

fn db_version() -> String {
    env::var("TEST_DB_VERSION").unwrap_or_else(|_| DEFAULT_DB_VERSION.to_string())
}

pub struct TestDbConfig {
    pub host: String,
    pub port: u16,
}

/// Runs the Liquibase migrations of the `liquibase-migrations` image against `container`.
///
/// # Panics
///
/// Panics if the container cannot be started or the migrations fail: a test cannot go on
/// without its schema.
pub async fn run_migrations(container: &ContainerAsync<GenericImage>) {
    let port = container
        .get_host_port_ipv4(POSTGRES_PORT.tcp())
        .await
        .expect("Port Postgres non exposé");
    let liquibase_url = format!("jdbc:postgresql://127.0.0.1:{port}/postgres");

    println!("🚀 Liquibase connectant à : {liquibase_url}");

    let liquibase_node = GenericImage::new("ghcr.io/mairie360/liquibase-migrations", &db_version())
        .with_network("host")
        .with_working_dir("/migrations") // Correspond au WORKDIR de ton Dockerfile
        .with_env_var("LIQUIBASE_SEARCH_PATH", "/migrations") // Comme dans ton Compose
        .with_cmd(vec![
            "update",
            "--url",
            &liquibase_url,
            "--username",
            "postgres",
            "--password",
            "postgres",
            "--changelog-file",
            "changelog.xml", // Relatif à /migrations
            // Since Database v1.7.0 (MAIR-413) the admin account (users.id = 1) is seeded from
            // -Dadmin_email / -Dadmin_password, or from the public template account with this
            // flag (dev and test stacks only); without either the migration stops half-way.
            "-Dallow_template_admin=true",
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

    // A failed migration leaves a half-built schema (missing functions, grants...): fail the setup
    // instead of letting every test fail later on an unrelated-looking error.
    let exit_code = liquibase_node
        .exit_code()
        .await
        .expect("Failed to read the Liquibase exit code");
    assert_eq!(
        exit_code,
        Some(0),
        "Liquibase migrations failed (image {}), see STDOUT/STDERR above",
        db_version()
    );

    println!("✅ Fin du container Liquibase.");
}

/// Démarre un conteneur Postgres standard
///
/// # Panics
///
/// Panique si le conteneur ou la base de test ne peut pas être préparé : un test ne peut pas
/// continuer sans son environnement.
pub async fn start_postgres_container() -> (ContainerAsync<GenericImage>, TestDbConfig) {
    // Port hôte aléatoire : un Postgres déjà présent sur 5432 (stack docker compose, conteneur
    // oublié…) ne peut plus prendre la place de la base de test.
    let node = GenericImage::new("ghcr.io/mairie360/database", &db_version())
        .with_exposed_port(POSTGRES_PORT.tcp())
        .with_env_var("POSTGRES_USER", "postgres")
        .with_env_var("POSTGRES_PASSWORD", "postgres")
        .with_env_var("POSTGRES_DB", "postgres")
        .start()
        .await
        .expect("Failed to start Postgres");

    let port = node
        .get_host_port_ipv4(POSTGRES_PORT.tcp())
        .await
        .expect("Port Postgres non exposé");
    let config = TestDbConfig {
        host: "127.0.0.1".to_string(),
        port,
    };

    let connection_string = format!(
        "host={} port={} user=postgres password=postgres dbname=postgres",
        config.host, config.port
    );

    // Healthcheck
    let mut ready = false;
    for _ in 0..HEALTHCHECK_ATTEMPTS {
        if tokio_postgres::connect(&connection_string, NoTls)
            .await
            .is_ok()
        {
            ready = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
    assert!(
        ready,
        "La base de test ne répond pas sur {}:{}",
        config.host, config.port
    );

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
