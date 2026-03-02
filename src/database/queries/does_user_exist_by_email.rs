use crate::database::db_interface::get_db_interface;
use crate::database::errors::DatabaseError;
use crate::database::postgresql::queries::DoesUserExistByEmailQuery;
use crate::database::queries_result_views::DoesUserExistByEmailQueryResultView;
use crate::database::query_views::DoesUserExistByEmailQueryView;

pub async fn does_user_exist_by_email_query(
    view: DoesUserExistByEmailQueryView,
) -> Result<DoesUserExistByEmailQueryResultView, DatabaseError> {
    let db_guard = get_db_interface().lock().unwrap();
    let db_interface = match &*db_guard {
        Some(db) => db,
        None => {
            eprintln!("Database interface is not initialized.");
            return Err(DatabaseError::NotInitialized);
        }
    };
    let query = DoesUserExistByEmailQuery::new(view.get_email());
    db_interface.execute_query(query).await
}
