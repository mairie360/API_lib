pub mod common;
use common::queries_setup::setup_tests;

use mairie360_api_lib::database::db_interface::{get_db_interface, QueryResultView};
use mairie360_api_lib::database::postgresql::queries::{
    AboutUserQuery, DoesUserExistByEmailQuery, DoesUserExistByIdQuery, LoginUserQuery, RegisterUserQuery
};
use mairie360_api_lib::database::postgresql::queries::errors::QueryError;
use mairie360_api_lib::database::queries_result_views::utils::QueryResult;
use serial_test::serial;
use serde_json::json;

// --- TESTS D'EXISTENCE ---

#[tokio::test]
#[serial]
async fn test_user_exists() {
    let _container = setup_tests().await;
    let query = DoesUserExistByIdQuery::new(1);

    let result = {
        let guard = get_db_interface().lock().unwrap();
        let db = guard.as_ref().unwrap();
        db.execute_query(query).await
    }.unwrap();

    assert_eq!(result.get_result(), QueryResult::Boolean(true));
}

#[tokio::test]
#[serial]
async fn test_user_not_found() {
    let _container = setup_tests().await;
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
    let _container = setup_tests().await;
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
async fn test_user_exists_by_email_invalid_format() {
    let _container = setup_tests().await;
    let email = "invalid-email";
    let query = DoesUserExistByEmailQuery::new(email);

    let result = {
        let guard = get_db_interface().lock().unwrap();
        let db = guard.as_ref().unwrap();
        db.execute_query(query).await
    };

    assert!(result.is_err());
    let err = result.err().unwrap();
    if let Some(query_err) = err.downcast_ref::<QueryError>() {
        assert_eq!(query_err, &QueryError::InvalidEmailFormat(email.to_string()));
    } else {
        panic!("Failed to downcast to QueryError");
    }
}

// --- TESTS DE LOGIN ---

#[tokio::test]
#[serial]
async fn test_login_user_success() {
    let _container = setup_tests().await;
    let query = LoginUserQuery::new("alice@example.com", "password123");

    let result = {
        let guard = get_db_interface().lock().unwrap();
        let db = guard.as_ref().unwrap();
        db.execute_query(query).await
    }.unwrap();

    assert_eq!(result.get_result(), QueryResult::U64(1));
}

#[tokio::test]
#[serial]
async fn test_login_user_wrong_password() {
    let _container = setup_tests().await;
    let email = "alice@example.com";
    let query = LoginUserQuery::new(email, "wrong_pass");

    let result = {
        let guard = get_db_interface().lock().unwrap();
        let db = guard.as_ref().unwrap();
        db.execute_query(query).await
    };

    assert!(result.is_err());
    let err = result.err().unwrap();
    if let Some(query_err) = err.downcast_ref::<QueryError>() {
        assert_eq!(query_err, &QueryError::InvalidPassword(email.to_string()));
    } else {
        panic!("Failed to downcast to QueryError");
    }
}

#[tokio::test]
#[serial]
async fn test_login_user_unknown_email() {
    let _container = setup_tests().await;
    let email = "stranger@danger.com";
    let query = LoginUserQuery::new(email, "any_password");

    let result = {
        let guard = get_db_interface().lock().unwrap();
        let db = guard.as_ref().unwrap();
        db.execute_query(query).await
    };

    assert!(result.is_err());
    let err = result.err().unwrap();
    if let Some(query_err) = err.downcast_ref::<QueryError>() {
        assert_eq!(query_err, &QueryError::EmailNotFound(email.to_string()));
    } else {
        panic!("Failed to downcast to QueryError");
    }
}

// --- TESTS DE CRÉATION ET CONSULTATION ---

#[tokio::test]
#[serial]
async fn test_register_user_success() {
    let _container = setup_tests().await;
    let email = "new_user@test.com";

    let register_query = RegisterUserQuery::new(
        "John", "Doe", email, "secure_password", Some("0601020304".to_string()),
    );

    let register_result = {
        let guard = get_db_interface().lock().unwrap();
        let db = guard.as_ref().unwrap();
        db.execute_query(register_query).await
    }.unwrap();

    assert_eq!(register_result.get_result(), QueryResult::Result(Ok(())));
}

#[tokio::test]
#[serial]
async fn test_about_user_success() {
    let _container = setup_tests().await;
    let query = AboutUserQuery::new(1);

    let result = {
        let guard = get_db_interface().lock().unwrap();
        let db = guard.as_ref().unwrap();
        db.execute_query(query).await
    }.unwrap();

    let expected_json = json!({
        "first_name": "Alice",
        "last_name": "Smith",
        "email": "alice@example.com",
        "phone": "0102030405",
        "status": "active"
    });

    assert_eq!(result.get_result(), QueryResult::JSON(expected_json));
}

#[tokio::test]
#[serial]
async fn test_about_user_fail() {
    let _container = setup_tests().await;
    let query = AboutUserQuery::new(999);

    let result = {
        let guard = get_db_interface().lock().unwrap();
        let db = guard.as_ref().unwrap();
        db.execute_query(query).await
    };

    assert!(result.is_err());
    let err = result.err().unwrap();
    if let Some(query_err) = err.downcast_ref::<QueryError>() {
        assert_eq!(
            query_err,
            &QueryError::InvalidId("User ID not found".to_string())
        );
    } else {
        panic!("Failed to downcast to QueryError");
    }
}
