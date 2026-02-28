use crate::database::db_interface::{DatabaseQueryView, Query};
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

    async fn execute(&self, client: &Client) -> Result<Self::Output, String> {
        let result = client
            .query_one(self.view.get_request().as_str(), &[])
            .await;
        match result {
            Ok(row) => {
                let exists: bool = row.get(0);
                Ok(DoesUserExistByEmailQueryResultView::new(exists))
            }
            Err(e) => Err(format!("Database query error: {}", e)),
        }
    }
}
