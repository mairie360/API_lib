use crate::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IsAdminQueryView {
    params: Vec<QueryParam>,
}

impl IsAdminQueryView {
    pub fn new(user_id: u64) -> Self {
        Self {
            params: vec![QueryParam::I32(user_id as i32)],
        }
    }
    pub fn get_user_id(&self) -> u64 {
        self.params[0].as_i32() as u64
    }
}

impl ApiRequestDto for IsAdminQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT is_admin($1)"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for IsAdminQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "IsAdminQueryView: user_id = {}", self.get_user_id())
    }
}
