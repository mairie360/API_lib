use crate::database::query_views::DoesUserExistByIdQueryView;
use crate::database::{db_interface::DatabaseQueryView, errors::DatabaseError};
use sqlx::PgPool;

pub async fn does_user_exist_by_id_query(
    view: DoesUserExistByIdQueryView,
    pool: PgPool,
) -> Result<bool, DatabaseError> {
    let result = sqlx::query_scalar::<_, bool>(&view.get_request())
        .bind(view.get_id() as i32)
        .fetch_one(&pool)
        .await?;

    Ok(result)
}
