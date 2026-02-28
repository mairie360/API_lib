use crate::database::db_interface::DatabaseQueryView;
use crate::database::QUERY;
use std::fmt::Display;

pub struct LoginUserQueryView {
    email: String,
    password: String,
    query: QUERY,
}

impl LoginUserQueryView {
    pub fn new(email: String, password: String) -> Self {
        Self {
            email,
            password,
            query: QUERY::LoginUser,
        }
    }

    pub fn get_email(&self) -> &String {
        &self.email
    }

    pub fn get_password(&self) -> &String {
        &self.password
    }
}

impl DatabaseQueryView for LoginUserQueryView {
    fn get_request(&self) -> String {
        "SELECT id, password FROM users WHERE email = $1".to_string()
    }
    fn get_raw_request(&self) -> String {
        format!(
            "SELECT id, password FROM users WHERE email = '{}'",
            self.email
        )
    }

    fn get_query_type(&self) -> QUERY {
        self.query
    }
}

impl Display for LoginUserQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "LoginUserQueryView: email = {}, password = [PROTECTED]",
            self.email
        )
    }
}
