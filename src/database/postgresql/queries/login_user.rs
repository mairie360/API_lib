use crate::database::db_interface::{DatabaseQueryView, Query};
use crate::database::queries_result_views::LoginUserQueryResultView;
use crate::database::query_views::LoginUserQueryView;
use async_trait::async_trait;
use tokio_postgres::Client;

pub struct LoginUserQuery {
    view: LoginUserQueryView,
}

impl LoginUserQuery {
    pub fn new(email: &str, password: &str) -> Self {
        Self {
            view: LoginUserQueryView::new(email.to_string(), password.to_string()),
        }
    }
}

#[async_trait]
impl Query for LoginUserQuery {
    type Output = LoginUserQueryResultView;

    async fn execute(&self, client: &Client) -> Result<Self::Output, String> {
        let result = client
            .query_one(self.view.get_request().as_str(), &[])
            .await;
        match result {
            Ok(row) => {
                let user_id = row.get::<&str, i32>("id") as u64;
                Ok(LoginUserQueryResultView::new(user_id))
            }
            Err(_) => {
                // Correspond à ta logique : renvoie un view avec ID 0 en cas d'erreur
                Ok(LoginUserQueryResultView::new(0))
            }
        }
    }
}
