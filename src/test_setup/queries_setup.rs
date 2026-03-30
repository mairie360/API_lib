use std::env;
use testcontainers::{ContainerAsync, GenericImage};
use tokio_postgres::{Client, NoTls};
use super::db_setup::start_postgres_container;

/// 1. Démarre le conteneur et expose le client SQL
pub async fn setup_test_container() -> (ContainerAsync<GenericImage>, Client, String) {
    let (node, _) = start_postgres_container().await;
    let host = "127.0.0.1";
    let port = 5432; 

    let postgres_url = format!("postgres://postgres:postgres@{}:{}/postgres", host, port);
    
    env::set_var("DB_HOST", host);
    env::set_var("DB_PORT", port.to_string());

    let (client, connection) = tokio_postgres::connect(&postgres_url, NoTls)
        .await
        .expect("Failed to connect to Postgres");

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    // Nettoyage initial systématique
    client.batch_execute("TRUNCATE TABLE users RESTART IDENTITY CASCADE;").await.unwrap();

    (node, client, postgres_url)
}

/// 2. Setup pour un utilisateur valide avec une session active
pub async fn setup_active_session(client: &Client) {
    client.batch_execute("
        INSERT INTO users (id, first_name, last_name, email, password, phone_number, status, is_archived)
        VALUES (1, 'Alice', 'Smith', 'alice@example.com', 'password123', '0102030405', 'active', FALSE);
        
        INSERT INTO sessions (user_id, user_is_archived, token_hash, ip_address, device_info)
        VALUES (1, FALSE, 'test_token_hash_unique_123', '127.0.0.1', 'Mozilla/5.0 (TestRunner)');
    ").await.expect("Failed to setup active session");
}

/// 3. Setup pour tester un token expiré
pub async fn setup_expired_session(client: &Client) {
    // Note : On utilise l'ID 1 supposant qu'Alice existe déjà ou on la crée si besoin
    client.batch_execute("
        INSERT INTO sessions (user_id, user_is_archived, token_hash, ip_address, device_info)
        VALUES (1, FALSE, 'test_token_hash_expired', '127.0.0.1', 'Mozilla/5.0');

        UPDATE sessions 
        SET expires_at = now() - INTERVAL '1 hour' 
        WHERE token_hash = 'test_token_hash_expired';
    ").await.expect("Failed to setup expired session");
}

/// 4. Setup pour tester un utilisateur archivé
pub async fn setup_archived_user_test(client: &Client) {
    client.batch_execute("
        -- 1. Créer Bob (Actif)
        INSERT INTO users (id, first_name, last_name, email, password, phone_number, status, is_archived)
        VALUES (2, 'Bob', 'Smith', 'bob@example.com', 'password123', '0102030405', 'active', FALSE);
        
        -- 2. Créer sa session (Liée à l'état non-archivé)
        INSERT INTO sessions (user_id, user_is_archived, token_hash, ip_address, device_info)
        VALUES (2, FALSE, 'test_token_hash_archived_user', '127.0.0.1', 'Mozilla/5.0');
        
        -- 3. CHANGER LES DEUX EN MÊME TEMPS
        -- On utilise un bloc qui garantit que la FK reste valide à la fin de l'opération
        UPDATE sessions SET user_is_archived = TRUE WHERE user_id = 2;
        UPDATE users SET is_archived = TRUE WHERE id = 2;
    ").await.expect("Failed to setup archived user test");
}

/// 5. Fonction globale si tu veux tout lancer d'un coup (ton ancienne approche)
pub async fn setup_tests_full() -> (ContainerAsync<GenericImage>, String) {
    let (node, client, url) = setup_test_container().await;
    setup_active_session(&client).await;
    setup_expired_session(&client).await;
    // setup_archived_user_test(&client).await;
    (node, url)
}
