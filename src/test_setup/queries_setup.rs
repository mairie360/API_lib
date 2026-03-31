use super::db_setup::start_postgres_container;
use std::{env, sync::OnceLock};
use testcontainers::{ContainerAsync, GenericImage};
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

/// 4. Setup pour tester un utilisateur archivé
pub async fn setup_archived_user_test(client: &Client) {
    client.batch_execute("
        -- 1. Créer Bob (Actif)
        INSERT INTO users (id, first_name, last_name, email, password, phone_number, status, is_archived)
        VALUES (2, 'Bob', 'Smith', 'bob@example.com', 'password123', '0102030405', 'active', FALSE)
        ON CONFLICT (id) DO UPDATE SET is_archived = FALSE;

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

/// 5. Setup des données d'accès contrôlé
pub async fn setup_access_control_data(client: &Client) {
    client
        .batch_execute(
            "
        -- 0. CRÉATION DES UTILISATEURS MANQUANTS (IMPORTANT)
        INSERT INTO users (id, first_name, last_name, email, password, status)
        VALUES (400, 'Admin', 'User', 'admin@test.com', 'hash', 'active'),
               (402, 'Bob', 'Guest', 'bob@test.com', 'hash', 'active')
        ON CONFLICT (id) DO NOTHING;

        -- 1. On s'assure que les ressources existent
        INSERT INTO resources (id, name) VALUES (1, 'user'), (2, 'groups'), (3, 'document')
        ON CONFLICT (id) DO NOTHING;

        -- 2. On s'assure que les permissions existent
        INSERT INTO permissions (id, resource_id, action) VALUES
        (1, 1, 'read_all'),
        (2, 3, 'read_all'),
        (3, 2, 'read')
        ON CONFLICT (id) DO NOTHING;

        -- 3. RBAC : Alice (1) est ADMIN
        INSERT INTO roles (id, name) VALUES (1, 'Admin') ON CONFLICT (id) DO NOTHING;
        INSERT INTO rights (role_id, permission_id) VALUES (1, 1), (1, 2) ON CONFLICT DO NOTHING;
        INSERT INTO user_roles (user_id, role_id) VALUES (1, 1) ON CONFLICT DO NOTHING;

        -- 4. Ownership : Alice (1) possède le document 10
        -- On vérifie si la table existe (cas où Liquibase n'aurait pas encore fini)
        CREATE TABLE IF NOT EXISTS document (id SERIAL PRIMARY KEY, owner_id INT);
        INSERT INTO document (id, owner_id) VALUES (10, 1)
        ON CONFLICT (id) DO UPDATE SET owner_id = EXCLUDED.owner_id;

        -- 5. ACL : Maintenant l'ID 400 existe, on peut créer le groupe
        INSERT INTO groups (id, owner_id, name, owner_is_archived)
        VALUES (50, 400, 'Seeded Group', false)
        ON CONFLICT (id) DO UPDATE SET owner_id = EXCLUDED.owner_id;

        -- Bob (402) accède au groupe 50
        INSERT INTO access_control (user_id, resource_id, permission_id, resource_instance_id)
        VALUES (402, 2, 3, 50) ON CONFLICT (id) DO NOTHING;
    ",
        )
        .await
        .expect("Failed to setup access control data");
}

/// 6. Fonction globale si tu veux tout lancer d'un coup (ton ancienne approche)
async fn setup_tests_full() -> (ContainerAsync<GenericImage>, String) {
    let (node, client, url) = setup_test_container().await;
    setup_active_session(&client).await;
    setup_expired_session(&client).await;
    setup_archived_user_test(&client).await;
    setup_access_control_data(&client).await;
    (node, url)
}

// On stocke le conteneur et l'URL pour qu'ils ne soient pas détruits
static SHARED_DB: OnceLock<(ContainerAsync<GenericImage>, String)> = OnceLock::new();

pub async fn get_shared_db() -> &'static (ContainerAsync<GenericImage>, String) {
    if let Some(db) = SHARED_DB.get() {
        return db;
    }

    // Premier appel : on initialise tout
    let (setup, host) = setup_tests_full().await;
    SHARED_DB.set((setup, host)).ok();
    SHARED_DB.get().unwrap()
}
