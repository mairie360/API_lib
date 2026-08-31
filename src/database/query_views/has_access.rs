use crate::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HasAccessQueryView {
    params: Vec<QueryParam>,
}

impl HasAccessQueryView {
    pub fn new(
        user_id: u64,
        p_resource_name: &str,
        p_action: &str,
        p_instance_id: Option<u64>,
    ) -> Self {
        Self {
            params: vec![
                QueryParam::I32(user_id as i32),
                QueryParam::Text(p_resource_name.to_string()),
                QueryParam::Text(p_action.to_string()),
                QueryParam::OptionI32(p_instance_id.map(|id| id as i32)),
            ],
        }
    }
    pub fn get_user_id(&self) -> u64 {
        self.params[0].as_i32() as u64
    }
    pub fn get_resource_name(&self) -> &str {
        self.params[1].as_text()
    }
    pub fn get_action(&self) -> &str {
        self.params[2].as_text()
    }
    pub fn get_instance_id(&self) -> Option<u64> {
        self.params[3].as_option_i32().map(|id| id as u64)
    }
}

impl ApiRequestDto for HasAccessQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT check_access($1, $2, $3, $4)"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for HasAccessQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "HasAccessQueryView: user_id = {}, resource_name = {}, action = {}, instance_id = {:?}",
            self.get_user_id(),
            self.get_resource_name(),
            self.get_action(),
            self.get_instance_id()
        )
    }
}
