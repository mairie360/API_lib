use crate::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DoesUserExistByEmailQueryView {
    params: Vec<QueryParam>,
}

impl DoesUserExistByEmailQueryView {
    pub fn new(email: String) -> Self {
        Self {
            params: vec![QueryParam::Text(email)],
        }
    }

    pub fn get_email(&self) -> &str {
        &self.params[0].as_text()
    }
}

impl ApiRequestDto for DoesUserExistByEmailQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT EXISTS(SELECT 1 FROM users WHERE email = $1) AS does_user_exist"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for DoesUserExistByEmailQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DoesUserExistByEmailQueryView: email = {}",
            self.get_email()
        )
    }
}
