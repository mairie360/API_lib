use std::env;
use testcontainers::{ContainerAsync, GenericImage};
use tokio_postgres::NoTls;
use super::db_setup::start_postgres_container;

pub async fn setup_tests() -> (ContainerAsync<GenericImage>, String) {
    // 1. Démarre le container (via ton db_setup qui est déjà en mode host)
    let (node, _) = start_postgres_container().await;

    // --- REMPLACE LES LIGNES 15 À 20 PAR CECI ---
    let host = "127.0.0.1";
    let port = 5432; 
    // On ne fait plus node.get_host() ni node.get_host_port_ipv4()
    // --------------------------------------------

    let postgres_url = format!("postgres://postgres:postgres@{}:{}/postgres", host, port);
    
    // On met à jour les variables d'environnement pour que ta lib sache où se connecter
    env::set_var("DB_HOST", host);
    env::set_var("DB_PORT", port.to_string());

    // Le reste du code (tokio_postgres::connect et client.batch_execute) est correct
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
            -- 1. Nettoyage propre avec CASCADE
            TRUNCATE TABLE users RESTART IDENTITY CASCADE;
    
            -- 2. Insertion avec les colonnes exactes de ta table
            -- (On a retiré 'username' qui n'existe pas dans ton DDL)
            INSERT INTO users (first_name, last_name, email, password, phone_number, status)
            VALUES (
                'Alice',
                'Smith',
                'alice@example.com',
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
