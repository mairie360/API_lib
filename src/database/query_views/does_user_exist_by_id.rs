use crate::database::db_interface::DatabaseQueryView;
use std::fmt::Display;

pub struct DoesUserExistByIdQueryView {
    id: u64,
}

impl DoesUserExistByIdQueryView {
    pub fn new(id: u64) -> Self {
        Self { id }
    }

    pub fn get_id(&self) -> u64 {
        self.id
    }
}

impl DatabaseQueryView for DoesUserExistByIdQueryView {
    fn get_request(&self) -> String {
        "SELECT EXISTS(SELECT 1 FROM users WHERE id = $1) AS does_user_exist".to_string()
    }
}

impl Display for DoesUserExistByIdQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DoesUserExistByIdQueryView: id = {}", self.id)
    }
}
