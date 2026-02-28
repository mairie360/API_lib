use mairie360_api_lib::database::db_interface::{get_db_interface, init_db_interface};
use mairie360_api_lib::database::postgresql::postgre_interface::{get_postgre_interface, reset_postgre_interface};
use testcontainers::GenericImage;
use testcontainers::runners::AsyncRunner;
use testcontainers::ImageExt;
use testcontainers::core::ContainerPort;
use testcontainers::ContainerAsync; // Ajout nécessaire pour le type de retour
use std::env;
use std::time::Duration;

/**
 * Setup a test database container and initialize the library interface.
 * Returns the ContainerAsync object to keep it alive during the test.
 */
pub async fn setup_test_db() -> ContainerAsync<GenericImage> {
    // 1. Start Container
    let node = GenericImage::new("postgres", "15-alpine")
        .with_exposed_port(ContainerPort::Tcp(5432))
        .with_env_var("POSTGRES_USER", "postgres")
        .with_env_var("POSTGRES_PASSWORD", "postgres")
        .with_env_var("POSTGRES_DB", "postgres")
        .start()
        .await
        .expect("Failed to start Postgres");

    let host = node.get_host().await.unwrap();
    let port = node.get_host_port_ipv4(5432).await.unwrap();

    // 2. Set Env
    env::set_var("DB_HOST", host.to_string());
    env::set_var("DB_PORT", port.to_string());
    env::set_var("DB_USER", "postgres");
    env::set_var("DB_PASSWORD", "postgres");
    env::set_var("DB_NAME", "postgres");

    // 3. Init Lib
    reset_postgre_interface().await;
    init_db_interface().await;

    // 4. Robust Connect with retry
    let mut connected = false;
    for _ in 0..10 {
        // Scope pour le lock afin d'éviter de le garder pendant le sleep
        let conn_attempt = {
            let mut guard = get_db_interface().lock().unwrap();
            if let Some(db) = guard.as_mut() {
                db.connect().await
            } else {
                Err("DbInterface not found".to_string())
            }
        };

        if conn_attempt.is_ok() {
            connected = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    assert!(connected, "Manual connection to test DB failed");

    // 5. Schema Setup
    {
        let p_guard = get_postgre_interface().await;
        let postgre = p_guard.as_ref().unwrap();
        let client_mutex = postgre.get_client();
        let locked_client = client_mutex.lock().await;
        let client = locked_client.as_ref().expect("Client tokio-postgres non connecté");

        client.batch_execute("
            CREATE TABLE IF NOT EXISTS users (id SERIAL PRIMARY KEY, username TEXT);
            INSERT INTO users (id, username) VALUES (1, 'Alice');
        ").await.expect("Failed to setup test schema");
    }

    node // On retourne l'objet node pour qu'il ne soit pas dropé (détruit)
}
