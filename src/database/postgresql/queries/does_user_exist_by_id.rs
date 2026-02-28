use crate::database::db_interface::{DatabaseQueryView, Query};
use crate::database::postgresql::queries::errors::QueryError;
use crate::database::queries_result_views::DoesUserExistByIdQueryResultView;
use crate::database::query_views::DoesUserExistByIdQueryView;
use async_trait::async_trait;
use tokio_postgres::Client;

pub struct DoesUserExistByIdQuery {
    view: DoesUserExistByIdQueryView,
}

impl DoesUserExistByIdQuery {
    pub fn new(user_id: u64) -> Self {
        Self {
            view: DoesUserExistByIdQueryView::new(user_id),
        }
    }
}

#[async_trait]
impl Query for DoesUserExistByIdQuery {
    type Output = DoesUserExistByIdQueryResultView;
    type Error = QueryError;

    async fn execute(&self, client: &Client) -> Result<Self::Output, Self::Error> {
        let result = client
            .query_one(self.view.get_request().as_str(), &[])
            .await;

        match result {
            Ok(row) => {
                let exists: bool = row.try_get(0)
                    .map_err(|e| QueryError::MappingError(e.to_string()))?;
                Ok(DoesUserExistByIdQueryResultView::new(exists))
            }
            Err(_) => Err(QueryError::InvalidId(self.view.get_id().to_string())),
        }
    }
}
