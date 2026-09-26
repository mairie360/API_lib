use super::db_setup::start_postgres_container;
use crate::password::hash_password;
use futures_util::FutureExt;
use std::env;
use std::panic::AssertUnwindSafe;
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use testcontainers::{ContainerAsync, GenericImage};
use tokio::sync::OnceCell;
use tokio_postgres::{Client, NoTls};

// Utilisation de OnceCell pour stocker les IDs récupérés dynamiquement
pub static ALICE_ID: OnceCell<i32> = OnceCell::const_new();
pub static BOB_ID: OnceCell<i32> = OnceCell::const_new();
pub static ADMIN_ID: OnceCell<i32> = OnceCell::const_new();
pub static GROUP_OWNER_ID: OnceCell<i32> = OnceCell::const_new();

/// Démarre le conteneur Postgres de test et renvoie le conteneur, un client et l'URL.
///
/// # Panics
///
/// Panique si le conteneur ou la base de test ne peut pas être préparé : un test ne peut pas
/// continuer sans son environnement.
pub async fn setup_test_container() -> (ContainerAsync<GenericImage>, Client, String) {
    let (node, config) = start_postgres_container().await;
    let postgres_url = format!(
        "postgres://postgres:postgres@{}:{}/postgres",
        config.host, config.port
    );

    env::set_var("DB_HOST", &config.host);
    env::set_var("DB_PORT", config.port.to_string());

    let (client, connection) = tokio_postgres::connect(&postgres_url, NoTls)
        .await
        .expect("Failed to connect to Postgres");

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {e}");
        }
    });

    (node, client, postgres_url)
}

/// Clear-text password of every seeded account.
pub const SEED_PASSWORD: &str = "password123";

/// Argon2id hash of [`SEED_PASSWORD`], computed once per process.
///
/// The `users.password` column only accepts argon2id PHC strings (`chk_users_password_hashed`,
/// database v1.3.0), so the fixtures cannot insert a clear-text password.
///
/// # Panics
///
/// Panics if the password cannot be hashed.
#[must_use]
pub fn seed_password_hash() -> &'static str {
    static HASH: OnceLock<String> = OnceLock::new();
    HASH.get_or_init(|| hash_password(SEED_PASSWORD).expect("Failed to hash the seed password"))
}

/// 2. Setup pour Alice (Utilisateur actif)
///
/// # Panics
///
/// Panique si le conteneur ou la base de test ne peut pas être préparé : un test ne peut pas
/// continuer sans son environnement.
pub async fn setup_active_session(client: &Client) {
    let row = client.query_one("
        INSERT INTO users (first_name, last_name, email, password, phone_number, status, is_archived)
        VALUES ('Alice', 'Smith', 'alice@example.com', $1, '0102030405', 'active', FALSE)
        ON CONFLICT (email) DO UPDATE SET email = EXCLUDED.email
        RETURNING id;
    ", &[&seed_password_hash()]).await.expect("Failed to insert Alice");

    let id: i32 = row.get(0);
    ALICE_ID.set(id).ok();

    client
        .execute(
            "
        INSERT INTO sessions (user_id, token_hash, ip_address, device_info)
        VALUES ($1, 'test_token_hash_unique_123', '127.0.0.1', 'Mozilla/5.0 (TestRunner)')
        ON CONFLICT DO NOTHING;
    ",
            &[&id],
        )
        .await
        .expect("Failed to setup active session");
}

/// 3. Setup Token expiré (réutilise Alice)
///
/// # Panics
///
/// Panique si le conteneur ou la base de test ne peut pas être préparé : un test ne peut pas
/// continuer sans son environnement.
pub async fn setup_expired_session(client: &Client) {
    let id = *ALICE_ID.get().expect("Alice ID not initialized");

    client.execute("
        INSERT INTO sessions (user_id, token_hash, ip_address, device_info, expires_at)
        VALUES ($1, 'test_token_hash_expired', '127.0.0.1', 'Mozilla/5.0', now() - INTERVAL '1 hour');
    ", &[&id]).await.expect("Failed to setup expired session");
}

/// 4. Setup pour Bob (Utilisateur qui finit archivé)
///
/// # Panics
///
/// Panique si le conteneur ou la base de test ne peut pas être préparé : un test ne peut pas
/// continuer sans son environnement.
pub async fn setup_archived_user_test(client: &Client) {
    let row = client.query_one("
        INSERT INTO users (first_name, last_name, email, password, phone_number, status, is_archived)
        VALUES ('Bob', 'Smith', 'bob@example.com', $1, '0102030405', 'active', FALSE)
        ON CONFLICT (email) DO UPDATE SET email = EXCLUDED.email
        RETURNING id;
    ", &[&seed_password_hash()]).await.expect("Failed to insert Bob");

    let id: i32 = row.get(0);
    BOB_ID.set(id).ok();

    // On utilise ON CONFLICT ici aussi pour la session
    client
        .execute(
            "
        INSERT INTO sessions (user_id, token_hash, ip_address, device_info)
        VALUES ($1, 'test_token_hash_archived_user', '127.0.0.1', 'Mozilla/5.0')
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
///
/// # Panics
///
/// Panique si le conteneur ou la base de test ne peut pas être préparé : un test ne peut pas
/// continuer sans son environnement.
pub async fn setup_access_control_data(client: &Client) {
    let alice_id = *ALICE_ID.get().expect("Alice ID missing");
    let bob_id = *BOB_ID.get().expect("Bob ID missing");

    // Admin avec ON CONFLICT
    let admin_row = client
        .query_one(
            "
        INSERT INTO users (first_name, last_name, email, password, status)
        VALUES ('Admin', 'User', 'admin@test.com', $1, 'active')
        ON CONFLICT (email) DO UPDATE SET email = EXCLUDED.email
        RETURNING id;
    ",
            &[&seed_password_hash()],
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
        VALUES ('Group', 'Owner', 'owner@test.com', $1, 'active')
        ON CONFLICT (email) DO UPDATE SET email = EXCLUDED.email
        RETURNING id;
    ",
            &[&seed_password_hash()],
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

        INSERT INTO groups (owner_id, name)
        VALUES ({owner_id}, 'Seeded Group') ON CONFLICT DO NOTHING;

        INSERT INTO access_control (user_id, resource_id, permission_id, resource_instance_id)
        VALUES ({bob_id}, 2, 3, 50) ON CONFLICT DO NOTHING;
    "
        ))
        .await
        .expect("Failed to setup access control data");
}

/// Outcome of the one-time setup, failure included: a `OnceCell` whose initialiser panics stays
/// empty, so every later test would start a new container and fail the same way again.
static SHARED_DB: OnceCell<Result<(ContainerAsync<GenericImage>, String), String>> =
    OnceCell::const_new();

/// Identifiant du conteneur de `SHARED_DB`, supprimé à la sortie du processus.
static SHARED_DB_CONTAINER_ID: OnceLock<String> = OnceLock::new();

/// Supprime le conteneur partagé : une `static` n'est jamais droppée, testcontainers ne le fait donc
/// pas lui-même, et le conteneur survivrait au binaire de test.
extern "C" fn remove_shared_db_container() {
    if let Some(id) = SHARED_DB_CONTAINER_ID.get() {
        let _ = Command::new("docker")
            .args(["rm", "--force", "--volumes", id])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

fn remove_shared_db_container_at_exit(id: &str) {
    if SHARED_DB_CONTAINER_ID.set(id.to_owned()).is_ok() {
        // SAFETY: `remove_shared_db_container` est une fonction `extern "C"` sans argument qui ne
        // panique pas ; `atexit` ne fait que l'enregistrer.
        // nosemgrep: rust.lang.security.unsafe-usage.unsafe-usage -- test-only helper, see SAFETY above
        if unsafe { libc::atexit(remove_shared_db_container) } != 0 {
            eprintln!("⚠️ Impossible d'enregistrer la suppression du conteneur de test {id}");
        }
    }
}

// async fn setup_tests_full() -> (ContainerAsync<GenericImage>, String) {
//     let (node, client, url) = setup_test_container().await;

//     // L'ordre est important pour que les OnceCell soient remplies
//     setup_active_session(&client).await;
//     setup_expired_session(&client).await;
//     setup_archived_user_test(&client).await;
//     setup_access_control_data(&client).await;

//     (node, url)
// }

/// Renvoie la base de test partagée par tous les tests du processus (créée au premier appel).
///
/// # Panics
///
/// Panique si le conteneur ou la base de test ne peut pas être préparé : un test ne peut pas
/// continuer sans son environnement.
pub async fn get_shared_db() -> &'static (ContainerAsync<GenericImage>, String) {
    match SHARED_DB
        .get_or_init(|| async {
            println!("🚀 Lancement du setup global UNIQUE...");
            AssertUnwindSafe(init_shared_db())
                .catch_unwind()
                .await
                .map_err(|payload| {
                    payload
                        .downcast_ref::<String>()
                        .cloned()
                        .or_else(|| payload.downcast_ref::<&str>().map(ToString::to_string))
                        .unwrap_or_else(|| "unknown panic".to_owned())
                })
        })
        .await
    {
        Ok(db) => db,
        Err(reason) => {
            panic!("The shared test database setup failed, nothing was retried: {reason}")
        }
    }
}

async fn init_shared_db() -> (ContainerAsync<GenericImage>, String) {
    // 1. Démarre le conteneur et le client
    let (node, client, url) = setup_test_container().await;
    remove_shared_db_container_at_exit(node.id());

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

    // 3. Insertions des données : exécutées une seule fois pour toute la durée des tests.
    setup_active_session(&client).await;
    setup_expired_session(&client).await;
    setup_archived_user_test(&client).await;
    setup_access_control_data(&client).await;

    println!("✅ Données de test injectées avec succès.");
    (node, url)
}
