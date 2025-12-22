use crate::database::db_interface::{DatabaseQueryView, QueryResultView};
use crate::database::queries_result_views::RegisterUserQueryResultView;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_postgres::Client;

/**
 * This function registers a user in the database.
 * It takes a query that implements the DatabaseQueryView trait,
 * and a client wrapped in an Arc and Mutex for thread safety.
 * It returns a Result containing a QueryResultView or an error message.
 * # Arguments
 * * `query`: A Box containing a query that implements the DatabaseQueryView trait.
 * * `client`: An Arc<Mutex<Option<Client>>> that holds the database client.
 * # Returns
 * * `Result<Box<dyn QueryResultView>, String>`: A Result containing a Box of QueryResultView on success,
 * or a String error message on failure.
 */
pub async fn register_user(
    query: Box<dyn DatabaseQueryView>,
    client: Arc<Mutex<Option<Client>>>,
) -> Result<Box<dyn QueryResultView>, String> {
    let tmp_client = client.lock().await;
    let client = tmp_client
        .as_ref()
        .ok_or("Database client is not initialized")?;
    let result = client.execute(query.get_request().as_str(), &[]).await;
    match result {
        Ok(1) => {
            // You can optionally check that _row_count == 1
            Ok(Box::new(RegisterUserQueryResultView::new(Ok(()))))
        }
        Ok(_) => {
            eprintln!("Unexpected number of rows affected, expected 1 but got more.");
            Err("Unexpected number of rows affected".to_string())
        }
        Err(e) => {
            eprintln!("Error executing query: {}", e);
            Err(format!("Database query error: {}", e))
        }
    }
}
