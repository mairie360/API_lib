use std::env;
use std::time::Duration;
use testcontainers::core::IntoContainerPort;
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, GenericImage, ImageExt};
use tokio_postgres::NoTls;

/// Default version of the `ghcr.io/mairie360/database` and `ghcr.io/mairie360/liquibase-migrations`
/// images used by the tests. The consumer services can override it with the `TEST_DB_VERSION`
/// environment variable (to test against another version than the lib's default).
///
/// Pinned to the latest published `dev-<sha>` image: the message change tracking and the task
/// indexes of MAIR-524 (Database `releases/v3.1.0`) are not in any tagged release yet.
const DEFAULT_DB_VERSION: &str = "dev-90c22bb";

/// Port interne du conteneur Postgres, publié sur un port hôte aléatoire.
const POSTGRES_PORT: u16 = 5432;

/// Nombre d'essais de connexion (espacés de 300 ms) avant de déclarer la base indisponible.
const HEALTHCHECK_ATTEMPTS: u32 = 100;

/// Login roles the migrations create, one per API.
///
/// They come from `repeatable/security/api_roles.sql` of Database (MAIR-114). [`run_migrations`]
/// gives each one [`TEST_API_ROLE_PASSWORD`] so the tests can connect as the API's role
/// (MAIR-556): a superuser ignores the `GRANT`s, so a query the role may not run would only fail
/// in staging.
pub const API_ROLES: [&str; 6] = [
    "core_api",
    "project_api",
    "calendar_api",
    "message_api",
    "elearning_api",
    "compliance_api",
];

/// Password of every role of [`API_ROLES`] in the test database.
pub const TEST_API_ROLE_PASSWORD: &str = "api-role-test-password";

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
    // -D<role>_password: without it the API roles stay NOLOGIN (MAIR-556). A Database version
    // that does not know a role ignores its parameter.
    let role_passwords: Vec<String> = API_ROLES
        .iter()
        .map(|role| format!("-D{role}_password={TEST_API_ROLE_PASSWORD}"))
        .collect();

    println!("🚀 Liquibase connectant à : {liquibase_url}");

    let liquibase_node = GenericImage::new("ghcr.io/mairie360/liquibase-migrations", &db_version())
        .with_network("host")
        .with_working_dir("/migrations") // Correspond au WORKDIR de ton Dockerfile
        .with_env_var("LIQUIBASE_SEARCH_PATH", "/migrations") // Comme dans ton Compose
        .with_cmd(
            [
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
            ]
            .into_iter()
            .chain(role_passwords.iter().map(String::as_str)),
        )
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
