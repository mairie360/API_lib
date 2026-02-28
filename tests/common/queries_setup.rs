use mairie360_api_lib::database::db_interface::{get_db_interface, init_db_interface};
use mairie360_api_lib::database::errors::DatabaseError;
use mairie360_api_lib::database::postgresql::postgre_interface::{get_postgre_interface, reset_postgre_interface};
use testcontainers::GenericImage;
use testcontainers::runners::AsyncRunner;
use testcontainers::ImageExt;
use testcontainers::core::ContainerPort;
use testcontainers::ContainerAsync;
use std::env;
use std::time::Duration;

/**
 * Setup a test database container and initialize the library interface.
 * Returns the ContainerAsync object to keep it alive during the test.
 */
pub async fn setup_tests() -> ContainerAsync<GenericImage> {
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

    // 2. Set Env pour la lib
    env::set_var("DB_HOST", host.to_string());
    env::set_var("DB_PORT", port.to_string());
    env::set_var("DB_USER", "postgres");
    env::set_var("DB_PASSWORD", "postgres");
    env::set_var("DB_NAME", "postgres");

    // 3. Init Lib : On reset l'interface pour s'assurer qu'elle recharge les variables d'env
    reset_postgre_interface().await;
    init_db_interface().await;

    // 4. Robust Connect with retry
    // On attend que Postgres soit réellement prêt à accepter des connexions
    let mut connected = false;
    for _ in 0..10 {
        let conn_attempt = {
            let mut guard = get_db_interface().lock().unwrap();
            if let Some(db) = guard.as_mut() {
                db.connect().await
            } else {
                Err(DatabaseError::Internal("PostgreInterface not initialized".to_string()))
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
            CREATE TABLE IF NOT EXISTS users (
                id SERIAL PRIMARY KEY,
                first_name TEXT,
                last_name TEXT,
                username TEXT,
                email TEXT UNIQUE,
                password TEXT,
                phone_number TEXT,
                status TEXT
            );

            -- Nettoyage pour l'idempotence des tests (indispensable avec serial_test)
            TRUNCATE TABLE users RESTART IDENTITY;

            -- On insère Alice avec TOUTES les infos pour valider AboutUser et Login
            INSERT INTO users (username, email, first_name, last_name, password, phone_number, status)
            VALUES (
                'Alice',
                'alice@example.com',
                'Alice',
                'Smith',
                'password123',
                '0102030405',
                'active'
            );
        ").await.expect("Failed to setup test schema");
    }

    node // On retourne l'objet node pour maintenir le conteneur en vie
}
