use crate::database::db_interface::{DatabaseQueryView, QueryResultView};
use crate::database::queries_result_views::DoesUserExistByEmailQueryResultView;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_postgres::Client;

/**
 * Checks if a user exists in the database by their email address.
 * This function takes a query object and a database client,
 * executes the query to check for the existence of a user with the given email,
 * and returns a result indicating whether the user exists.
 * # Arguments
 * * `query`: A boxed trait object implementing `DatabaseQueryView`, which contains the SQL query to execute.
 * * `client`: An `Arc<Mutex<Option<Client>>>` that holds the database client.
 * # Returns
 * * `Result<Box<dyn QueryResultView>, String>`: A result containing a boxed trait object implementing `QueryResultView` if the query is successful,
 * or an error message if the query fails.
 */
pub async fn does_user_exist_by_email(
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
            Ok(Box::new(DoesUserExistByEmailQueryResultView::new(exists)))
        }
        Err(e) => {
            eprintln!("Error executing query: {}", e);
            Err(format!("Database query error: {}", e))
        }
    }
}
