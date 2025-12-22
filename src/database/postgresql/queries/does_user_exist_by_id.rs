use crate::database::db_interface::{DatabaseQueryView, QueryResultView};
use crate::database::queries_result_views::DoesUserExistByIdQueryResultView;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_postgres::Client;

/**
 * Checks if a user exists in the database by their ID.
 *
 * # Arguments
 * * `query` - A boxed trait object implementing `DatabaseQueryView` that contains the SQL query.
 * * `client` - An `Arc<Mutex<Option<Client>>>` that holds the database client.
 *
 * # Returns
 * * `Result<Box<dyn QueryResultView>, String>` - A result containing a boxed trait object implementing `QueryResultView` if the user exists, or an error message if the query
 * fails.
 */
pub async fn does_user_exist_by_id(
    query: Box<dyn DatabaseQueryView>,
    client: Arc<Mutex<Option<Client>>>,
) -> Result<Box<dyn QueryResultView>, String> {
    let tmp_client = client.lock().await;
    let client = tmp_client
        .as_ref()
        .ok_or("Database client is not initialized")?;
    let result = client.query_one(query.get_request().as_str(), &[]).await;
    match result {
        Ok(row) => {
            let exists: bool = row.get(0);
            Ok(Box::new(DoesUserExistByIdQueryResultView::new(exists)))
        }
        Err(e) => {
            eprintln!("Error executing query: {}", e);
            Err(format!("Database query error: {}", e))
        }
    }
}
