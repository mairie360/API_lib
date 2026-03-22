use crate::database::queries_result_views::DoesUserExistByIdQueryResultView;
use crate::database::query_views::DoesUserExistByIdQueryView;
use crate::database::{db_interface::DatabaseQueryView, errors::DatabaseError};
use sqlx::PgPool;

pub async fn does_user_exist_by_id_query(
    view: DoesUserExistByIdQueryView,
    pool: PgPool,
) -> Result<DoesUserExistByIdQueryResultView, DatabaseError> {
    let result = sqlx::query_as::<_, DoesUserExistByIdQueryResultView>(&view.get_request())
        .bind(view.get_id() as i32)
        .fetch_one(&pool)
        .await?;

    Ok(result)
}
