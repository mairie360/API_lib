pub mod common;
use common::db_setup::{set_db_env_vars, start_postgres_container};
use mairie360_api_lib::database::db_interface::{get_db_interface, init_db_interface};
use mairie360_api_lib::database::errors::DatabaseError;
use mairie360_api_lib::database::postgresql::postgre_interface::reset_postgre_interface;
use serial_test::serial;

// Helper interne pour gérer le lock empoisonné sans crash
fn clear_poison() {
    let lock = get_db_interface().lock();
    if let Err(poisoned) = lock {
        // On récupère la donnée malgré le poisoning pour "nettoyer" l'état
        let mut guard = poisoned.into_inner();
        *guard = None;
    }
}

#[tokio::test]
#[serial]
async fn test_interface_connection_success() {
    let (_container, config) = start_postgres_container().await;
    set_db_env_vars(&config, "postgres", "postgres", "postgres");

    clear_poison(); // Sécurité
    reset_postgre_interface().await;
    init_db_interface().await;

    // On utilise un scope pour que le lock soit relâché le plus vite possible
    let res = {
        let mut guard = get_db_interface().lock().unwrap_or_else(|e| e.into_inner());
        let db = guard.as_mut().unwrap();
        db.connect().await
    };

    assert!(res.is_ok(), "La connexion a échoué : {:?}", res.err());
}

#[tokio::test]
#[serial]
async fn test_interface_connection_fail_wrong_password() {
    let (_container, config) = start_postgres_container().await;
    set_db_env_vars(&config, "postgres", "postgres", "MAUVAIS_PASS");

    clear_poison();
    reset_postgre_interface().await;
    init_db_interface().await;

    let res = {
        let mut guard = get_db_interface().lock().unwrap_or_else(|e| e.into_inner());
        let db = guard.as_mut().unwrap();
        db.connect().await
    };

    assert!(
        res.is_err(),
        "La connexion aurait dû échouer avec un mauvais mot de passe"
    );

    if let Err(DatabaseError::ConnectionFailed(m)) = res {
        // On vérifie que le message mentionne un problème d'authentification ou de connexion
        let m_lower = m.to_lowercase();
        assert!(
            m_lower.contains("authentication failed")
                || m_lower.contains("password")
                || m_lower.contains("failed to connect"),
            "Message d'erreur inattendu : {}",
            m
        );
    } else {
        panic!("Type d'erreur inattendu : {:?}", res.err());
    }
}

#[tokio::test]
#[serial]
async fn test_interface_execute_without_connection() {
    let (_container, config) = start_postgres_container().await;
    set_db_env_vars(&config, "postgres", "postgres", "postgres");

    clear_poison();
    reset_postgre_interface().await;
    init_db_interface().await;

    let res = {
        let guard = get_db_interface().lock().unwrap_or_else(|e| e.into_inner());
        let db = guard.as_ref().unwrap();

        use mairie360_api_lib::database::postgresql::queries::DoesUserExistByIdQuery;
        db.execute_query(DoesUserExistByIdQuery::new(1)).await
    };

    assert!(res.is_err());
}

#[tokio::test]
#[serial]
async fn test_interface_full_session_flow() {
    // 1. Setup : Initialisation propre
    let (_container, config) = start_postgres_container().await;
    set_db_env_vars(&config, "postgres", "postgres", "postgres");

    clear_poison();
    reset_postgre_interface().await;
    init_db_interface().await;

    // 2. Action : Login (Connect)
    let login_result = {
        let mut guard = get_db_interface().lock().unwrap_or_else(|e| e.into_inner());
        let db = guard.as_mut().unwrap();
        db.connect().await
    };

    assert!(login_result.is_ok(), "Le Login (connect) a échoué");
    assert_eq!(login_result.unwrap(), "PostgreSQL Connected");

    // 3. Action : Logout (Disconnect)
    // On ré-ouvre le lock pour simuler une action ultérieure
    let logout_result = {
        let mut guard = get_db_interface().lock().unwrap_or_else(|e| e.into_inner());
        let db = guard.as_mut().unwrap();
        db.disconnect().await
    };

    // 4. Assertions
    assert!(logout_result.is_ok(), "Le Logout (disconnect) a échoué");
    assert_eq!(logout_result.unwrap(), "PostgreSql Disconnected");
}
