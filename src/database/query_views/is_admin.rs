use crate::database::db_interface::DatabaseQueryView;
use std::fmt::Display;

pub struct IsAdminQueryView {
    user_id: u64,
}

impl IsAdminQueryView {
    pub fn new(user_id: u64) -> Self {
        Self { user_id }
    }
    pub fn get_user_id(&self) -> u64 {
        self.user_id
    }
}

impl DatabaseQueryView for IsAdminQueryView {
    fn get_request(&self) -> String {
        "SELECT is_admin($1)".to_string()
    }
}

impl Display for IsAdminQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "IsAdminQueryView: user_id = {}", self.user_id)
    }
}
