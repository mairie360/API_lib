use crate::database::db_interface::{id_from_sql, id_to_sql, ApiRequestDto, QueryParam};
use std::fmt::Display;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IsAdminQueryView {
    params: Vec<QueryParam>,
}

impl IsAdminQueryView {
    #[must_use]
    pub fn new(user_id: u64) -> Self {
        Self {
            params: vec![QueryParam::I32(id_to_sql(user_id))],
        }
    }
    #[must_use]
    pub fn get_user_id(&self) -> u64 {
        id_from_sql(self.params[0].as_i32())
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
