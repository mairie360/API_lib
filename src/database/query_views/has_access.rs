use crate::database::db_interface::DatabaseQueryView;
use std::fmt::Display;

pub struct HasAccessQueryView {
    user_id: u64,
    p_resource_name: String,
    p_action: String,
    p_instance_id: u64,
}

impl HasAccessQueryView {
    pub fn new(user_id: u64, p_resource_name: &str, p_action: &str, p_instance_id: u64) -> Self {
        Self {
            user_id,
            p_resource_name: p_resource_name.to_string(),
            p_action: p_action.to_string(),
            p_instance_id,
        }
    }
    pub fn get_user_id(&self) -> u64 {
        self.user_id
    }
    pub fn get_resource_name(&self) -> &str {
        &self.p_resource_name
    }
    pub fn get_action(&self) -> &str {
        &self.p_action
    }
    pub fn get_instance_id(&self) -> u64 {
        self.p_instance_id
    }
}

impl DatabaseQueryView for HasAccessQueryView {
    fn get_request(&self) -> String {
        "SELECT check_access($1, $2, $3, $4)".to_string()
    }
}

impl Display for HasAccessQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "HasAccessQueryView: user_id = {}, resource_name = {}, action = {}, instance_id = {}",
            self.user_id, self.p_resource_name, self.p_action, self.p_instance_id
        )
    }
}
