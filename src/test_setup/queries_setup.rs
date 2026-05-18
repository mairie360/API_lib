use super::db_setup::start_postgres_container;
use std::env;
use testcontainers::{ContainerAsync, GenericImage};
use tokio::sync::OnceCell;
use tokio_postgres::{Client, NoTls};

// Utilisation de OnceCell pour stocker les IDs récupérés dynamiquement
pub static ALICE_ID: OnceCell<i32> = OnceCell::const_new();
pub static BOB_ID: OnceCell<i32> = OnceCell::const_new();
pub static ADMIN_ID: OnceCell<i32> = OnceCell::const_new();
pub static GROUP_OWNER_ID: OnceCell<i32> = OnceCell::const_new();

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

    (node, client, postgres_url)
}

/// 2. Setup pour Alice (Utilisateur actif)
pub async fn setup_active_session(client: &Client) {
    let row = client.query_one("
        INSERT INTO users (first_name, last_name, email, password, phone_number, status, is_archived)
        VALUES ('Alice', 'Smith', 'alice@example.com', 'password123', '0102030405', 'active', FALSE)
        ON CONFLICT (email) DO UPDATE SET email = EXCLUDED.email
        RETURNING id;
    ", &[]).await.expect("Failed to insert Alice");

    let id: i32 = row.get(0);
    ALICE_ID.set(id).ok();

    client
        .execute(
            "
        INSERT INTO sessions (user_id, user_is_archived, token_hash, ip_address, device_info)
        VALUES ($1, FALSE, 'test_token_hash_unique_123', '127.0.0.1', 'Mozilla/5.0 (TestRunner)')
        ON CONFLICT DO NOTHING;
    ",
            &[&id],
        )
        .await
        .expect("Failed to setup active session");
}

/// 3. Setup Token expiré (réutilise Alice)
pub async fn setup_expired_session(client: &Client) {
    let id = *ALICE_ID.get().expect("Alice ID not initialized");

    client.execute("
        INSERT INTO sessions (user_id, user_is_archived, token_hash, ip_address, device_info, expires_at)
        VALUES ($1, FALSE, 'test_token_hash_expired', '127.0.0.1', 'Mozilla/5.0', now() - INTERVAL '1 hour');
    ", &[&id]).await.expect("Failed to setup expired session");
}

/// 4. Setup pour Bob (Utilisateur qui finit archivé)
pub async fn setup_archived_user_test(client: &Client) {
    let row = client.query_one("
        INSERT INTO users (first_name, last_name, email, password, phone_number, status, is_archived)
        VALUES ('Bob', 'Smith', 'bob@example.com', 'password123', '0102030405', 'active', FALSE)
        ON CONFLICT (email) DO UPDATE SET email = EXCLUDED.email
        RETURNING id;
    ", &[]).await.expect("Failed to insert Bob");

    let id: i32 = row.get(0);
    BOB_ID.set(id).ok();

    // On utilise ON CONFLICT ici aussi pour la session
    client
        .execute(
            "
        INSERT INTO sessions (user_id, user_is_archived, token_hash, ip_address, device_info)
        VALUES ($1, FALSE, 'test_token_hash_archived_user', '127.0.0.1', 'Mozilla/5.0')
        ON CONFLICT DO NOTHING;
    ",
            &[&id],
        )
        .await
        .expect("Failed to setup Bob session");

    client
        .execute(
            "UPDATE users SET is_archived = TRUE, status = 'archived' WHERE id = $1",
            &[&id],
        )
        .await
        .expect("Failed to archive Bob");
}

/// 5. Setup des données d'accès (Utilise un nouvel utilisateur pour les groupes)
pub async fn setup_access_control_data(client: &Client) {
    let alice_id = *ALICE_ID.get().expect("Alice ID missing");
    let bob_id = *BOB_ID.get().expect("Bob ID missing");

    // Admin avec ON CONFLICT
    let admin_row = client
        .query_one(
            "
        INSERT INTO users (first_name, last_name, email, password, status)
        VALUES ('Admin', 'User', 'admin@test.com', 'hash', 'active')
        ON CONFLICT (email) DO UPDATE SET email = EXCLUDED.email
        RETURNING id;
    ",
            &[],
        )
        .await
        .expect("Failed to insert Admin");
    let admin_id: i32 = admin_row.get(0);

    client
        .batch_execute(&format!(
            "INSERT INTO user_roles (user_id, role_id) VALUES ({admin_id}, 1) ON CONFLICT DO NOTHING;"
        ))
        .await
        .expect("Failed to insert User Role");
    ADMIN_ID.set(admin_id).ok();

    // Group Owner avec ON CONFLICT
    let owner_row = client
        .query_one(
            "
        INSERT INTO users (first_name, last_name, email, password, status)
        VALUES ('Group', 'Owner', 'owner@test.com', 'hash', 'active')
        ON CONFLICT (email) DO UPDATE SET email = EXCLUDED.email
        RETURNING id;
    ",
            &[],
        )
        .await
        .expect("Failed to insert Group Owner");
    let owner_id: i32 = owner_row.get(0);
    GROUP_OWNER_ID.set(owner_id).ok();

    // Pour les rôles et ACL, on peut utiliser des subqueries ou des batchs simples
    // Ici on reste sur ton format batch_execute mais on s'assure de ne pas recréer la table document à chaque fois
    client
        .batch_execute(&format!(
            "
        INSERT INTO user_roles (user_id, role_id) VALUES ({alice_id}, 1) ON CONFLICT DO NOTHING;

        CREATE TABLE IF NOT EXISTS document (id SERIAL PRIMARY KEY, owner_id INT);
        INSERT INTO document (owner_id) VALUES ({alice_id});

        INSERT INTO groups (owner_id, name, owner_is_archived)
        VALUES ({owner_id}, 'Seeded Group', false) ON CONFLICT DO NOTHING;

        INSERT INTO access_control (user_id, resource_id, permission_id, resource_instance_id)
        VALUES ({bob_id}, 2, 3, 50) ON CONFLICT DO NOTHING;
    "
        ))
        .await
        .expect("Failed to setup access control data");
}

static SHARED_DB: OnceCell<(ContainerAsync<GenericImage>, String)> = OnceCell::const_new();

// async fn setup_tests_full() -> (ContainerAsync<GenericImage>, String) {
//     let (node, client, url) = setup_test_container().await;

//     // L'ordre est important pour que les OnceCell soient remplies
//     setup_active_session(&client).await;
//     setup_expired_session(&client).await;
//     setup_archived_user_test(&client).await;
//     setup_access_control_data(&client).await;

//     (node, url)
// }

pub async fn get_shared_db() -> &'static (ContainerAsync<GenericImage>, String) {
    SHARED_DB
        .get_or_init(|| async {
            println!("🚀 Lancement du setup global UNIQUE...");

            // 1. Démarre le conteneur et le client
            let (node, client, url) = setup_test_container().await;

            // 2. Nettoie les données existantes (sans supprimer les tables)
            client
                .batch_execute(
                    "
                TRUNCATE TABLE
                    access_control,
                    user_roles,
                    groups,
                    sessions,
                    users
                RESTART IDENTITY CASCADE;
            ",
                )
                .await
                .expect("Erreur lors du nettoyage des données");

            // 3. ICI on lance les insertions de données.
            // Comme on est dans le OnceCell, ce bloc ne s'exécutera qu'UNE FOIS
            // pour toute la durée de tes tests.
            setup_active_session(&client).await;
            setup_expired_session(&client).await;
            setup_archived_user_test(&client).await;
            setup_access_control_data(&client).await;

            println!("✅ Données de test injectées avec succès.");
            (node, url)
        })
        .await
}
