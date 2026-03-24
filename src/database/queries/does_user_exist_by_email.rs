use crate::database::db_interface::DatabaseQueryView;
use crate::database::errors::DatabaseError;
use crate::database::queries::QueryError;
use crate::database::query_views::DoesUserExistByEmailQueryView;
use sqlx::PgPool;

pub async fn does_user_exist_by_email_query(
    view: DoesUserExistByEmailQueryView,
    pool: PgPool,
) -> Result<bool, DatabaseError> {
    if !view.get_email().contains('@') {
        return Err(DatabaseError::Query(QueryError::InvalidEmailFormat(
            view.get_email().to_string(),
        )));
    }

    let result = sqlx::query_scalar::<_, bool>(&view.get_request())
        .bind(view.get_email())
        .fetch_one(&pool)
        .await?;

    Ok(result)
}
