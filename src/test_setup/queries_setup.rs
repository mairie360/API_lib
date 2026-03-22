use std::env;
use testcontainers::core::{ContainerPort, WaitFor};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, GenericImage, ImageExt};
use tokio_postgres::NoTls;

pub async fn setup_tests() -> (ContainerAsync<GenericImage>, String) {
    // 1. L'ordre est crucial : d'abord le port, ensuite l'env et le wait
    let node = GenericImage::new("postgres", "15-alpine")
        .with_wait_for(WaitFor::message_on_stderr(
            "database system is ready to accept connections",
        ))
        .with_exposed_port(ContainerPort::Tcp(5432))
        .with_env_var("POSTGRES_USER", "postgres")
        .with_env_var("POSTGRES_PASSWORD", "postgres")
        .with_env_var("POSTGRES_DB", "postgres")
        .start()
        .await
        .expect("Failed to start Postgres");

    let host = node.get_host().await.expect("Failed to get host");
    // On utilise bien le port 5432 exposé pour récupérer le port dynamique mappé par Docker
    let port = node
        .get_host_port_ipv4(5432)
        .await
        .expect("Failed to get port");

    // 2. Connexion et Setup (Ton code reste identique ici)
    let postgres_url = format!("postgres://postgres:postgres@{}:{}/postgres", host, port);
    env::set_var("DB_HOST", host.to_string());
    env::set_var("DB_PORT", port.to_string());

    let (client, connection) = tokio_postgres::connect(&postgres_url, NoTls)
        .await
        .expect("Failed to connect to Postgres");

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    client
        .batch_execute(
            "
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
    ",
        )
        .await
        .expect("Failed to setup test schema");

    (node, postgres_url)
}
