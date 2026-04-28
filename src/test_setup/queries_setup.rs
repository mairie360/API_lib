use super::db_setup::start_postgres_container;
use std::env;
use testcontainers::{ContainerAsync, GenericImage};
use tokio::sync::OnceCell;
use tokio_postgres::{Client, NoTls};

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
    client
        .batch_execute(
            "
        -- On vide TOUTES les tables impactées par le seeding
        TRUNCATE TABLE
            access_control,
            rights,
            user_roles,
            roles,
            permissions,
            resources,
            sessions,
            users
        RESTART IDENTITY CASCADE;
    ",
        )
        .await
        .unwrap();

    (node, client, postgres_url)
}

pub static ALICE_ID: u64 = 1;

/// 2. Setup pour un utilisateur valide avec une session active
pub async fn setup_active_session(client: &Client) {
    client.batch_execute("
        INSERT INTO users (first_name, last_name, email, password, phone_number, status, is_archived)
        VALUES ('Alice', 'Smith', 'alice@example.com', 'password123', '0102030405', 'active', FALSE);

        INSERT INTO sessions (user_id, user_is_archived, token_hash, ip_address, device_info)
        VALUES (1, FALSE, 'test_token_hash_unique_123', '127.0.0.1', 'Mozilla/5.0 (TestRunner)');
    ").await.expect("Failed to setup active session");
}

/// 3. Setup pour tester un token expiré
pub async fn setup_expired_session(client: &Client) {
    // Note : On utilise l'ID 1 supposant qu'Alice existe déjà ou on la crée si besoin
    client
        .batch_execute(
            "
        INSERT INTO sessions (user_id, user_is_archived, token_hash, ip_address, device_info)
        VALUES (1, FALSE, 'test_token_hash_expired', '127.0.0.1', 'Mozilla/5.0');

        UPDATE sessions
        SET expires_at = now() - INTERVAL '1 hour'
        WHERE token_hash = 'test_token_hash_expired';
    ",
        )
        .await
        .expect("Failed to setup expired session");
}

pub static BOB_ID: u64 = 1;

/// 4. Setup pour tester un utilisateur archivé
pub async fn setup_archived_user_test(client: &Client) {
    client.batch_execute("
        -- 1. Créer Bob (Actif)
        INSERT INTO users (first_name, last_name, email, password, phone_number, status, is_archived)
        VALUES ('Bob', 'Smith', 'bob@example.com', 'password123', '0102030405', 'active', FALSE);

        -- 2. Créer sa session (Active)
        INSERT INTO sessions (user_id, user_is_archived, token_hash, ip_address, device_info)
        VALUES (2, FALSE, 'test_token_hash_archived_user', '127.0.0.1', 'Mozilla/5.0');

        -- 3. ARCHIVAGE : Si ton CHECK interdit 'TRUE' dans sessions,
        -- tu dois soit supprimer la session, soit modifier la contrainte.
        -- Ici, on simule l'archivage de l'user.
        UPDATE users SET is_archived = TRUE, status = 'archived' WHERE id = 2;

        -- NOTE: Si 'user_is_archived' dans la table sessions a un CHECK à FALSE,
        -- cette ligne suivante échouera TOUJOURS :
        -- UPDATE sessions SET user_is_archived = TRUE WHERE user_id = 2;
    ").await.expect("Failed to setup archived user test");
}

pub static ADMIN_ID: u64 = 3;

/// 5. Setup des données d'accès contrôlé
pub async fn setup_access_control_data(client: &Client) {
    client
        .batch_execute(
            "
        -- 0. CRÉATION DES UTILISATEURS MANQUANTS (IMPORTANT)
        INSERT INTO users (first_name, last_name, email, password, status)
        VALUES ('Admin', 'User', 'admin@test.com', 'hash', 'active');

        -- 1. On s'assure que les ressources existent
        INSERT INTO resources (name) VALUES ('user'), ('groups'), ('document');

        -- 2. On s'assure que les permissions existent
        INSERT INTO permissions (resource_id, action) VALUES
        (1, 'read_all'),
        (2, 'read_all'),
        (3, 'read');

        -- 3. RBAC : Alice (1) est ADMIN
        INSERT INTO roles (name) VALUES ('Admin');
        INSERT INTO rights (role_id, permission_id) VALUES (1, 1), (1, 2);
        INSERT INTO user_roles (user_id, role_id) VALUES (1, 1);

        -- 4. Ownership : Alice (1) possède le document 10
        -- On vérifie si la table existe (cas où Liquibase n'aurait pas encore fini)
        CREATE TABLE IF NOT EXISTS document (id SERIAL PRIMARY KEY, owner_id INT);
        INSERT INTO document (owner_id) VALUES (1);

        -- 5. ACL : Maintenant l'ID 3 existe, on peut créer le groupe
        INSERT INTO groups (owner_id, name, owner_is_archived)
        VALUES (3, 'Seeded Group', false);

        -- Bob (2) accède au groupe 50
        INSERT INTO access_control (user_id, resource_id, permission_id, resource_instance_id)
        VALUES (2, 2, 3, 50);
    ",
        )
        .await
        .expect("Failed to setup access control data");
}

// On stocke le conteneur et l'URL pour qu'ils ne soient pas détruits
static SHARED_DB: OnceCell<(ContainerAsync<GenericImage>, String)> = OnceCell::const_new();

/// 6. Fonction globale si tu veux tout lancer d'un coup (ton ancienne approche)
async fn setup_tests_full() -> (ContainerAsync<GenericImage>, String) {
    let (node, client, url) = setup_test_container().await;
    setup_active_session(&client).await;
    setup_expired_session(&client).await;
    setup_archived_user_test(&client).await;
    setup_access_control_data(&client).await;
    (node, url)
}

pub async fn get_shared_db() -> &'static (ContainerAsync<GenericImage>, String) {
    SHARED_DB
        .get_or_init(|| async {
            println!("🚀 Initialisation unique de la DB de test...");

            setup_tests_full().await
        })
        .await
}
