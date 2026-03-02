use crate::database::db_interface::DatabaseQueryView;
use std::fmt::Display;

pub struct DoesUserExistByEmailQueryView {
    email: String,
}

impl DoesUserExistByEmailQueryView {
    pub fn new(email: String) -> Self {
        Self { email }
    }

    pub fn get_email(&self) -> &String {
        &self.email
    }
}

impl DatabaseQueryView for DoesUserExistByEmailQueryView {
    fn get_request(&self) -> String {
        "SELECT EXISTS(SELECT 1 FROM users WHERE email = $1)".to_string()
    }
    fn get_raw_request(&self) -> String {
        format!(
            "SELECT EXISTS(SELECT 1 FROM users WHERE email = '{}')",
            self.email
        )
    }
}

impl Display for DoesUserExistByEmailQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DoesUserExistByEmailQueryView: email = {}", self.email)
    }
}
