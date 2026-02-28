mod common;

use mairie360_api_lib::database::db_interface::get_db_interface;
use mairie360_api_lib::database::postgresql::queries::DoesUserExistByIdQuery;
use mairie360_api_lib::database::queries_result_views::utils::get_boolean_from_query_result;
use mairie360_api_lib::database::db_interface::QueryResultView;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_user_exists() {
    // Le conteneur vivra jusqu'à la fin de cette fonction
    let _container = common::setup_test_db().await;

    let query = DoesUserExistByIdQuery::new(1);

    let result = {
        let guard = get_db_interface().lock().unwrap();
        let db = guard.as_ref().unwrap();
        db.execute_query(query).await
    };

    assert!(result.is_ok());
    assert!(get_boolean_from_query_result(result.unwrap().get_result()));
}

#[tokio::test]
#[serial]
async fn test_user_not_found() {
    // Le conteneur vivra jusqu'à la fin de cette fonction
    let _container = common::setup_test_db().await;

    let query = DoesUserExistByIdQuery::new(999);

    let result = {
        let guard = get_db_interface().lock().unwrap();
        let db = guard.as_ref().unwrap();
        db.execute_query(query).await
    }.unwrap();

    assert!(!get_boolean_from_query_result(result.get_result()));
}