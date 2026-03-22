use mairie360_api_lib::test_setup::queries_setup::setup_tests;

use mairie360_api_lib::database::db_interface::QueryResultView;
use mairie360_api_lib::database::queries::QueryError;
use mairie360_api_lib::database::queries::{
    does_user_exist_by_email_query, does_user_exist_by_id_query,
};
use mairie360_api_lib::database::queries_result_views::utils::QueryResult;
use mairie360_api_lib::database::query_views::{
    DoesUserExistByEmailQueryView, DoesUserExistByIdQueryView,
};
use serial_test::serial;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

#[cfg(test)]
mod queries_tests {
    use mairie360_api_lib::database::errors::DatabaseError;

    use super::*;

    async fn get_pool(url: String) -> PgPool {
        PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(std::time::Duration::from_secs(3))
            .connect(&url) // On passe l'URL construite ici
            .await
            .expect("Failed to create Postgres pool")
    }

    // --- TESTS D'EXISTENCE ---

    #[tokio::test]
    #[serial]
    async fn test_user_exists() {
        let (_container, host) = setup_tests().await;
        let pool = get_pool(host).await;
        let view = DoesUserExistByIdQueryView::new(1);

        let result = does_user_exist_by_id_query(view, pool).await.unwrap();

        assert_eq!(result.get_result(), QueryResult::Boolean(true));
    }

    #[tokio::test]
    #[serial]
    async fn test_user_not_found() {
        let (_container, host) = setup_tests().await;
        let pool = get_pool(host).await;
        let view = DoesUserExistByIdQueryView::new(999);

        let result = does_user_exist_by_id_query(view, pool).await.unwrap();

        assert_eq!(result.get_result(), QueryResult::Boolean(false));
    }

    #[tokio::test]
    #[serial]
    async fn test_user_exists_by_email() {
        let (_container, host) = setup_tests().await;
        let pool = get_pool(host).await;
        let view = DoesUserExistByEmailQueryView::new("alice@example.com".to_string());

        let result = does_user_exist_by_email_query(view, pool).await.unwrap();

        assert_eq!(result.get_result(), QueryResult::Boolean(true));
    }

    #[tokio::test]
    #[serial]
    async fn test_user_exists_by_email_invalid_format() {
        let (_container, host) = setup_tests().await;
        let pool = get_pool(host).await;
        let email = "invalid-email";
        let view = DoesUserExistByEmailQueryView::new(email.to_string());

        let result = does_user_exist_by_email_query(view, pool).await;

        assert!(result.is_err());
        let err = result.err().unwrap();
        println!("Received error: {:?}", err);
        assert_eq!(
            err,
            DatabaseError::Query(QueryError::InvalidEmailFormat(email.to_string()))
        );
    }
}
