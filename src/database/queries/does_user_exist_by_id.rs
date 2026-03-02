use crate::database::db_interface::get_db_interface;
use crate::database::errors::DatabaseError;
use crate::database::postgresql::queries::DoesUserExistByIdQuery;
use crate::database::queries_result_views::DoesUserExistByIdQueryResultView;
use crate::database::query_views::DoesUserExistByIdQueryView;

pub async fn does_user_exist_by_id_query(
    view: DoesUserExistByIdQueryView,
) -> Result<DoesUserExistByIdQueryResultView, DatabaseError> {
    let db_guard = get_db_interface().lock().unwrap();
    let db_interface = match &*db_guard {
        Some(db) => db,
        None => {
            eprintln!("Database interface is not initialized.");
            return Err(DatabaseError::NotInitialized);
        }
    };
    let query = DoesUserExistByIdQuery::new(*view.get_id());
    let query_view = db_interface.execute_query(query).await;
    match query_view {
        Ok(view) => Ok(view),
        Err(e) => {
            eprintln!("Database error occurred: {}", e);
            Err(DatabaseError::Internal(
                "Failed to execute AboutUserQuery".to_string(),
            ))
        }
    }
}
