use crate::database::db_interface::{DatabaseQueryView, Query};
use crate::database::postgresql::queries::errors::QueryError;
use crate::database::queries_result_views::AboutUserQueryResultView;
use crate::database::query_views::AboutUserQueryView;
use async_trait::async_trait;
use tokio_postgres::Client;

pub struct AboutUserQuery {
    view: AboutUserQueryView,
}

impl AboutUserQuery {
    pub fn new(user_id: u64) -> Self {
        Self {
            view: AboutUserQueryView::new(user_id),
        }
    }
}

#[async_trait]
impl Query for AboutUserQuery {
    type Output = AboutUserQueryResultView;
    type Error = QueryError;

    async fn execute(&self, client: &Client) -> Result<Self::Output, Self::Error> {
        // 1. On force le type i64 (BIGINT Postgres)
        let id = *self.view.get_id() as i32;

        // 2. On passe l'ID en spécifiant bien la référence ToSql
        let result = client
            .query_one(
                self.view.get_request().as_str(),
                &[&id as &(dyn tokio_postgres::types::ToSql + Sync)]
            )
            .await;

        match result {
            Ok(row) => {
                let first_name: &str = row.try_get("first_name").map_err(|e| QueryError::MappingError(e.to_string()))?;
                let last_name: &str = row.try_get("last_name").map_err(|e| QueryError::MappingError(e.to_string()))?;
                let email: &str = row.try_get("email").map_err(|e| QueryError::MappingError(e.to_string()))?;
                let phone: &str = row.try_get("phone_number").map_err(|e| QueryError::MappingError(e.to_string()))?;
                let status: &str = row.try_get("status").map_err(|e| QueryError::MappingError(e.to_string()))?;

                Ok(AboutUserQueryResultView::new(first_name, last_name, email, phone, status))
            }
            Err(e) => {
                let err_msg = e.to_string();
                if err_msg.contains("0 rows") || err_msg.contains("unexpected number of rows") {
                    Err(QueryError::InvalidId("User ID not found".to_string()))
                } else {
                    Err(QueryError::from(e))
                }
            }
        }
    }
}
