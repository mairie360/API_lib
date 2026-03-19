use crate::database::db_interface::DatabaseQueryView;
use crate::database::errors::DatabaseError;
use crate::database::queries::QueryError;
use crate::database::queries_result_views::DoesUserExistByEmailQueryResultView;
use crate::database::query_views::DoesUserExistByEmailQueryView;
use sqlx::PgPool;

pub async fn does_user_exist_by_email_query(
    view: DoesUserExistByEmailQueryView,
    pool: PgPool,
) -> Result<DoesUserExistByEmailQueryResultView, DatabaseError> {
    if !view.get_email().contains('@') {
        return Err(DatabaseError::Query(QueryError::InvalidEmailFormat(
            view.get_email().to_string(),
        )));
    }

    let result = sqlx::query_as::<_, DoesUserExistByEmailQueryResultView>(&view.get_request())
        .bind(view.get_email())
        .fetch_one(&pool)
        .await?;

    Ok(result)
}
