use crate::database::db_interface::{DatabaseQueryView, Query};
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

    async fn execute(&self, client: &Client) -> Result<Self::Output, String> {
        let result = client
            .query_one(self.view.get_request().as_str(), &[])
            .await;
        match result {
            Ok(row) => {
                // Utilisation des noms de colonnes pour la clarté
                Ok(AboutUserQueryResultView::new(
                    row.get("first_name"),
                    row.get("last_name"),
                    row.get("email"),
                    row.get("phone_number"),
                    row.get("status"),
                ))
            }
            Err(e) => Err(format!("Database query error: {}", e)),
        }
    }
}
