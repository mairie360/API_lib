use crate::database::query_views::IsAdminQueryView;
use crate::database::{db_interface::DatabaseQueryView, errors::DatabaseError};
use sqlx::PgPool;

pub async fn is_admin_query(view: IsAdminQueryView, pool: PgPool) -> Result<bool, DatabaseError> {
    let result = sqlx::query_scalar::<_, bool>(&view.get_request())
        .bind(view.get_user_id() as i32)
        .fetch_one(&pool)
        .await?;

    Ok(result)
}
