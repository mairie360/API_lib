use crate::database::db_interface::{DatabaseQueryView, QueryResultView};
use crate::database::queries_result_views::LoginUserQueryResultView;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_postgres::Client;

/**
 * Function to login a user by executing a query against the PostgreSQL database.
 * It retrieves the user ID from the query result and returns it wrapped in a `LoginUserQueryResultView`.
 * # Arguments
 * * `query`: A boxed trait object implementing `DatabaseQueryView`, which contains the SQL query to be executed.
 * * `client`: An `Arc<Mutex<Option<Client>>>` that provides access to the PostgreSQL client.
 * # Returns
 * * `Result<Box<dyn QueryResultView>, String>`: On success, it returns a boxed trait object containing the result of the query.
 *   On failure, it returns an error message as a `String`.
 */
pub async fn login_user(
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
            let user_id = row.get::<&str, i32>("id") as u64;
            Ok(Box::new(LoginUserQueryResultView::new(user_id)))
        }
        Err(e) => {
            eprintln!("Error executing query: {}", e);
            Ok(Box::new(LoginUserQueryResultView::new(0)))
        }
    }
}
