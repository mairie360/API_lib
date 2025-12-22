use crate::database::db_interface::get_db_interface;
use crate::database::queries_result_views::get_boolean_from_query_result;
use crate::database::query_views::DoesUserExistByIdQueryView;

/**
 * Checks if a user exists in the database by their ID.
 *
 * # Arguments
 * `user_id` - The ID of the user to check for existence.
 *
 * # Returns
 * `true` if the user exists, `false` otherwise.
 */
pub async fn does_user_exist_by_id(user_id: u64) -> bool {
    let view = DoesUserExistByIdQueryView::new(user_id);
    let db_guard = get_db_interface().lock().unwrap();
    let db_interface = match &*db_guard {
        Some(db) => db,
        None => {
            return false;
        }
    };
    let query_view = db_interface.execute_query(Box::new(view)).await;

    let result = match query_view {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Error executing query: {}", e);
            return false;
        }
    };

    if !get_boolean_from_query_result(result.get_result()) {
        return false;
    }
    true
}
