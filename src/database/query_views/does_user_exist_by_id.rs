use crate::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DoesUserExistByIdQueryView {
    params: Vec<QueryParam>,
}

impl DoesUserExistByIdQueryView {
    pub fn new(id: u64) -> Self {
        Self {
            params: vec![QueryParam::I32(id as i32)],
        }
    }

    pub fn get_id(&self) -> u64 {
        self.params[0].as_i32() as u64
    }
}

impl ApiRequestDto for DoesUserExistByIdQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT EXISTS(SELECT 1 FROM users WHERE id = $1) AS does_user_exist"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for DoesUserExistByIdQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DoesUserExistByIdQueryView: id = {}", self.get_id())
    }
}
