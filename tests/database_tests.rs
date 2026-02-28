mod common;

use mairie360_api_lib::database::db_interface::{get_db_interface, QueryResultView};
use mairie360_api_lib::database::postgresql::queries::{
    AboutUserQuery, DoesUserExistByEmailQuery, DoesUserExistByIdQuery, LoginUserQuery, RegisterUserQuery
};
use mairie360_api_lib::database::queries_result_views::utils::QueryResult;
use serial_test::serial;
use serde_json::json;

#[tokio::test]
#[serial]
async fn test_user_exists() {
    let _container = common::setup_test_db().await;
    let query = DoesUserExistByIdQuery::new(1);

    let result = {
        let guard = get_db_interface().lock().unwrap();
        let db = guard.as_ref().unwrap();
        db.execute_query(query).await
    }.unwrap();

    // Comparaison directe de l'enum QueryResult
    assert_eq!(result.get_result(), QueryResult::Boolean(true));
}

#[tokio::test]
#[serial]
async fn test_user_not_found() {
    let _container = common::setup_test_db().await;
    let query = DoesUserExistByIdQuery::new(999);

    let result = {
        let guard = get_db_interface().lock().unwrap();
        let db = guard.as_ref().unwrap();
        db.execute_query(query).await
    }.unwrap();

    assert_eq!(result.get_result(), QueryResult::Boolean(false));
}

#[tokio::test]
#[serial]
async fn test_user_exists_by_email() {
    let _container = common::setup_test_db().await;
    let query = DoesUserExistByEmailQuery::new("alice@example.com");

    let result = {
        let guard = get_db_interface().lock().unwrap();
        let db = guard.as_ref().unwrap();
        db.execute_query(query).await
    }.unwrap();

    assert_eq!(result.get_result(), QueryResult::Boolean(true));
}

#[tokio::test]
#[serial]
async fn test_register_user_success() {
    let _container = common::setup_test_db().await;
    let email = "new_user@test.com";

    let register_query = RegisterUserQuery::new(
        "John", "Doe", email, "secure_password", Some("0601020304".to_string()),
    );

    let register_result = {
        let guard = get_db_interface().lock().unwrap();
        let db = guard.as_ref().unwrap();
        db.execute_query(register_query).await
    }.unwrap();

    // Vérifie que le résultat de l'enregistrement est un Ok(()) encapsulé
    assert_eq!(register_result.get_result(), QueryResult::Result(Ok(())));
}

#[tokio::test]
#[serial]
async fn test_login_user_success() {
    let _container = common::setup_test_db().await;
    let query = LoginUserQuery::new("alice@example.com", "password123");

    let result = {
        let guard = get_db_interface().lock().unwrap();
        let db = guard.as_ref().unwrap();
        db.execute_query(query).await
    }.unwrap();

    // On attend l'ID 1 (U64) pour Alice
    assert_eq!(result.get_result(), QueryResult::U64(1));
}

#[tokio::test]
#[serial]
async fn test_login_user_wrong_password() {
    let _container = common::setup_test_db().await;
    let query = LoginUserQuery::new("alice@example.com", "wrong_pass");

    let result = {
        let guard = get_db_interface().lock().unwrap();
        let db = guard.as_ref().unwrap();
        db.execute_query(query).await
    }.unwrap();

    // Ta logique renvoie 0 en cas d'échec de login
    assert_eq!(result.get_result(), QueryResult::U64(0));
}

#[tokio::test]
#[serial]
async fn test_login_user_unknown_email() {
    // 1. Setup : Initialise la base avec Alice (ID 1) uniquement
    let _container = common::setup_test_db().await;

    // 2. Action : Tentative de login avec un email inexistant
    let query = LoginUserQuery::new("stranger@danger.com", "any_password");

    let result = {
        let guard = get_db_interface().lock().unwrap();
        let db = guard.as_ref().unwrap();
        db.execute_query(query).await
    };

    // 3. Assertions
    // La requête doit réussir (pas d'erreur de communication)
    assert!(result.is_ok());

    // Mais le résultat doit être l'ID 0 (Utilisateur non trouvé)
    assert_eq!(result.unwrap().get_result(), QueryResult::U64(0));
}

#[tokio::test]
#[serial]
async fn test_about_user_success() {
    let _container = common::setup_test_db().await;
    let query = AboutUserQuery::new(1);

    let result = {
        let guard = get_db_interface().lock().unwrap();
        let db = guard.as_ref().unwrap();
        db.execute_query(query).await
    }.unwrap();

    // On construit le JSON attendu pour comparer l'enum QueryResult::JSON
    let expected_json = json!({
        "first_name": "Alice",
        "last_name": "Smith",
        "email": "alice@example.com",
        "phone": "0102030405",
        "status": "active"
    });

    assert_eq!(result.get_result(), QueryResult::JSON(expected_json));
}
