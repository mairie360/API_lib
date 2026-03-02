use crate::database::db_interface::{DatabaseQueryView, Query};
use crate::database::errors::DatabaseError;
use crate::database::queries::QueryError;
use crate::database::queries_result_views::DoesUserExistByEmailQueryResultView;
use crate::database::query_views::DoesUserExistByEmailQueryView;
use async_trait::async_trait;
use tokio_postgres::Client;

pub struct DoesUserExistByEmailQuery {
    view: DoesUserExistByEmailQueryView,
}

impl DoesUserExistByEmailQuery {
    pub fn new(email: &str) -> Self {
        Self {
            view: DoesUserExistByEmailQueryView::new(email.to_string()),
        }
    }
}

#[async_trait]
impl Query for DoesUserExistByEmailQuery {
    type Output = DoesUserExistByEmailQueryResultView;

    async fn execute(&self, client: &Client) -> Result<Self::Output, DatabaseError> {
        if !self.view.get_email().contains('@') {
            return Err(DatabaseError::Query(QueryError::InvalidEmailFormat(
                self.view.get_email().clone(),
            )));
        }

        let email = self.view.get_email();
        let result = client
            .query_one(self.view.get_request().as_str(), &[&email])
            .await;

        match result {
            Ok(row) => {
                let exists: bool = row
                    .try_get(0)
                    .map_err(|e| DatabaseError::Query(QueryError::MappingError(e.to_string())))?;

                Ok(DoesUserExistByEmailQueryResultView::new(exists))
            }
            Err(e) => Err(DatabaseError::from(e)),
        }
    }
}
