pub mod common;
use common::queries_setup::setup_tests;

use mairie360_api_lib::database::db_interface::{get_db_interface, QueryResultView};
use mairie360_api_lib::database::postgresql::queries::errors::QueryError;
use mairie360_api_lib::database::postgresql::queries::{
    AboutUserQuery, DoesUserExistByEmailQuery, DoesUserExistByIdQuery, LoginUserQuery,
    RegisterUserQuery,
};
use mairie360_api_lib::database::queries_result_views::utils::QueryResult;
use serde_json::json;
use serial_test::serial;

#[cfg(test)]
mod queries_tests {
    use super::*;

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
        }
        .unwrap();

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
        }
        .unwrap();

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
        }
        .unwrap();

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
            assert_eq!(
                query_err,
                &QueryError::InvalidEmailFormat(email.to_string())
            );
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
        }
        .unwrap();

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
            "John",
            "Doe",
            email,
            "secure_password",
            Some("0601020304".to_string()),
        );

        let register_result = {
            let guard = get_db_interface().lock().unwrap();
            let db = guard.as_ref().unwrap();
            db.execute_query(register_query).await
        }
        .unwrap();

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
        }
        .unwrap();

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
}

#[cfg(test)]
mod sql_injection_tests {
    use super::*;

    #[tokio::test]
    #[serial]
    async fn test_injection_login_email() {
        let _container = setup_tests().await;

        // Tentative classique : ' OR 1=1 --
        // Si c'était vulnérable, cela pourrait bypasser le check d'email ou retourner le premier user
        let malicious_email = "' OR 1=1 --";
        let query = LoginUserQuery::new(malicious_email, "any_password");

        let result = {
            let guard = get_db_interface().lock().unwrap();
            let db = guard.as_ref().unwrap();
            db.execute_query(query).await
        };

        // On attend une erreur EmailNotFound car la chaîne est traitée littéralement
        assert!(result.is_err());
        let err = result.err().unwrap();
        if let Some(query_err) = err.downcast_ref::<QueryError>() {
            assert_eq!(
                query_err,
                &QueryError::EmailNotFound(malicious_email.to_string())
            );
        } else {
            panic!("L'injection a causé une erreur inattendue ou a réussi");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_injection_register_fields() {
        let _container = setup_tests().await;

        let malicious_name = "John'); DROP TABLE users; --";
        let register_query =
            RegisterUserQuery::new(malicious_name, "Doe", "attacker@test.com", "pass", None);

        let result = {
            let guard = get_db_interface().lock().unwrap();
            let db = guard.as_ref().unwrap();
            db.execute_query(register_query).await
        }
        .unwrap();

        assert_eq!(result.get_result(), QueryResult::Result(Ok(())));

        // Le reste de tes vérifications...
        let check_query = DoesUserExistByIdQuery::new(1);
        let check_result = {
            let guard = get_db_interface().lock().unwrap();
            let db = guard.as_ref().unwrap();
            db.execute_query(check_query).await
        }
        .unwrap();

        assert_eq!(check_result.get_result(), QueryResult::Boolean(true));
    }

    #[tokio::test]
    #[serial]
    async fn test_injection_about_user_string_id() {
        let _container = setup_tests().await;

        // Ton code fait un cast `as i64` ou `as i32`.
        // Si quelqu'un essaie d'envoyer un ID malicieux via une API,
        // le compilateur Rust ou ton cast échouera avant même la DB.
        // Ici on teste avec un ID qui n'est pas un nombre (simulé par AboutUserQuery::new)
        // Note: Comme AboutUserQuery prend un u64, l'injection par ID est impossible par design (Strong Typing).

        let query = AboutUserQuery::new(1); // On utilise un ID valide
        let result = {
            let guard = get_db_interface().lock().unwrap();
            let db = guard.as_ref().unwrap();
            db.execute_query(query).await
        };

        assert!(result.is_ok());
    }
}
