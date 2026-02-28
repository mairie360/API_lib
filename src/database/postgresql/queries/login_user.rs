use crate::database::db_interface::{DatabaseQueryView, Query};
use crate::database::postgresql::queries::errors::QueryError;
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
    type Error = QueryError;

    async fn execute(&self, client: &Client) -> Result<Self::Output, Self::Error> {
        // 1. On cherche l'utilisateur par email uniquement
        let email = self.view.get_email();
        let result = client
            .query_opt(self.view.get_request().as_str(), &[&email])
            .await;

        match result {
            Ok(Some(row)) => {
                // L'utilisateur existe, on extrait l'ID et le mot de passe stocké
                let user_id: i32 = row
                    .try_get("id")
                    .map_err(|e| QueryError::MappingError(e.to_string()))?;

                let db_password: String = row
                    .try_get("password")
                    .map_err(|e| QueryError::MappingError(e.to_string()))?;

                // 2. Comparaison du mot de passe
                if db_password == *self.view.get_password() {
                    Ok(LoginUserQueryResultView::new(user_id as u64))
                } else {
                    // Email trouvé, mais mot de passe incorrect
                    Err(QueryError::InvalidPassword(self.view.get_email().clone()))
                }
            }
            Ok(None) => {
                // Aucune ligne renvoyée : l'email n'existe pas en base
                Err(QueryError::EmailNotFound(self.view.get_email().clone()))
            }
            Err(e) => Err(QueryError::from(e)),
        }
    }
}
