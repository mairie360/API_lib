use crate::database::query_views::HasAccessQueryView;
use crate::database::{db_interface::DatabaseQueryView, errors::DatabaseError};
use sqlx::PgPool;

pub async fn has_access_query(
    view: HasAccessQueryView,
    pool: PgPool,
) -> Result<bool, DatabaseError> {
    let result = sqlx::query_scalar::<_, bool>(&view.get_request())
        .bind(view.get_user_id() as i32)
        .bind(view.get_resource_name())
        .bind(view.get_action())
        .bind(view.get_instance_id() as i32)
        .fetch_one(&pool)
        .await?;

    Ok(result)
}
