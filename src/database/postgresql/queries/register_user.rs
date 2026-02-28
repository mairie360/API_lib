use crate::database::db_interface::{DatabaseQueryView, Query};
use crate::database::postgresql::queries::errors::QueryError;
use crate::database::queries_result_views::RegisterUserQueryResultView;
use crate::database::query_views::RegisterUserQueryView;
use async_trait::async_trait;
use tokio_postgres::Client;

pub struct RegisterUserQuery {
    view: RegisterUserQueryView,
}

impl RegisterUserQuery {
    pub fn new(first: &str, last: &str, email: &str, pass: &str, phone: Option<String>) -> Self {
        Self {
            view: RegisterUserQueryView::new(
                first.to_string(),
                last.to_string(),
                email.to_string(),
                pass.to_string(),
                phone,
            ),
        }
    }
}

#[async_trait]
impl Query for RegisterUserQuery {
    type Output = RegisterUserQueryResultView;
    type Error = QueryError;

    async fn execute(&self, client: &Client) -> Result<Self::Output, Self::Error> {
        let result = match self.view.get_phone_number() {
            Some(phone) => {
                client
                    .execute(
                        self.view.get_request().as_str(),
                        &[
                            &self.view.get_first_name().as_str(), // $1: &str
                            &self.view.get_last_name().as_str(),  // $2: &str
                            &self.view.get_email().as_str(),      // $3: &str
                            &self.view.get_password().as_str(),   // $4: &str
                            &phone.as_str(),                      // $5: &str
                        ],
                    )
                    .await
            }
            None => {
                client
                    .execute(
                        self.view.get_request().as_str(),
                        &[
                            &self.view.get_first_name().as_str(),
                            &self.view.get_last_name().as_str(),
                            &self.view.get_email().as_str(),
                            &self.view.get_password().as_str(),
                        ],
                    )
                    .await
            }
        };

        match result {
            Ok(1) => Ok(RegisterUserQueryResultView::new(Ok(()))),
            Ok(actual) => Err(QueryError::AffectedRowsMismatch {
                expected: 1,
                actual,
            }),
            Err(e) => Err(QueryError::from(e)),
        }
    }
}
