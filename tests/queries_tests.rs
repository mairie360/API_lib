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

#[cfg(test)]
mod queries_tests {
    use mairie360_api_lib::database::errors::DatabaseError;

    use super::*;

    // --- TESTS D'EXISTENCE ---

    #[tokio::test]
    #[serial]
    async fn test_user_exists() {
        let _container = setup_tests().await;
        let view = DoesUserExistByIdQueryView::new(1);

        let result = does_user_exist_by_id_query(view).await.unwrap();

        assert_eq!(result.get_result(), QueryResult::Boolean(true));
    }

    #[tokio::test]
    #[serial]
    async fn test_user_not_found() {
        let _container = setup_tests().await;
        let view = DoesUserExistByIdQueryView::new(999);

        let result = does_user_exist_by_id_query(view).await.unwrap();

        assert_eq!(result.get_result(), QueryResult::Boolean(false));
    }

    #[tokio::test]
    #[serial]
    async fn test_user_exists_by_email() {
        let _container = setup_tests().await;
        let view = DoesUserExistByEmailQueryView::new("alice@example.com".to_string());

        let result = does_user_exist_by_email_query(view).await.unwrap();

        assert_eq!(result.get_result(), QueryResult::Boolean(true));
    }

    #[tokio::test]
    #[serial]
    async fn test_user_exists_by_email_invalid_format() {
        let _container = setup_tests().await;
        let email = "invalid-email";
        let view = DoesUserExistByEmailQueryView::new(email.to_string());

        let result = does_user_exist_by_email_query(view).await;

        assert!(result.is_err());
        let err = result.err().unwrap();
        println!("Received error: {:?}", err);
        assert_eq!(
            err,
            DatabaseError::Query(QueryError::InvalidEmailFormat(email.to_string()))
        );
    }
}
