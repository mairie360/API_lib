use crate::database::db_interface::{DatabaseQueryView, QueryResultView};
use crate::database::queries_result_views::AboutUserQueryResultView;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_postgres::Client;

/**
 * This function executes a query to retrieve information about a user.
 *
 * It takes a query object that implements the `DatabaseQueryView` trait and a client wrapped in an `Arc<Mutex<Option<Client>>>`.
 * The function locks the client, executes the query, and retrieves user details such as first name, last name, email, phone number, and status.
 * If the query is successful, it returns a boxed `AboutUserQueryResultView` containing the user details.
 * If there is an error during query execution, it returns an error message.
 */
pub async fn about_user(
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
            let first_name: &str = row.get("first_name");
            let last_name: &str = row.get("last_name");
            let email: &str = row.get("email");
            let phone: &str = row.get("phone_number");
            let status: &str = row.get("status");

            Ok(Box::new(AboutUserQueryResultView::new(
                first_name, last_name, email, phone, status,
            )))
        }
        Err(e) => {
            eprintln!("Error executing query: {}", e);
            Err(format!("Database query error: {}", e))
        }
    }
}
