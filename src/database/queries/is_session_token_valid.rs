use crate::database::query_views::IsSessionTokenValidQueryView;
use crate::database::{db_interface::DatabaseQueryView, errors::DatabaseError};
use sqlx::PgPool;

pub async fn is_session_token_valid_query(
    view: IsSessionTokenValidQueryView,
    pool: PgPool,
) -> Result<bool, DatabaseError> {
    let result = sqlx::query_scalar::<_, bool>(&view.get_request())
        .bind(view.get_user_id() as i32)
        .bind(view.get_session_token())
        .bind(view.get_ip_address())
        .fetch_one(&pool)
        .await?;

    Ok(result)
}
