use crate::database::db_interface::DatabaseQueryView;
use crate::database::QUERY;
use std::fmt::Display;

pub struct AboutUserQueryView {
    id: u64,
    query: QUERY,
}

impl AboutUserQueryView {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            query: QUERY::AboutUser,
        }
    }

    pub fn get_id(&self) -> &u64 {
        &self.id
    }
}

impl DatabaseQueryView for AboutUserQueryView {
    fn get_request(&self) -> String {
        "SELECT first_name, last_name, email, phone_number, status FROM users WHERE id = $1"
            .to_string()
    }
    fn get_raw_request(&self) -> String {
        format!(
            "SELECT first_name, last_name, email, phone_number, status FROM users WHERE id = '{}'",
            self.id
        )
    }

    fn get_query_type(&self) -> QUERY {
        self.query
    }
}

impl Display for AboutUserQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AboutUserQueryView: id = {}", self.id)
    }
}
