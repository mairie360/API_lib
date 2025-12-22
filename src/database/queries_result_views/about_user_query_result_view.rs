use super::QueryResult;
use crate::database::db_interface::QueryResultView;
use serde::{Deserialize, Serialize};
use serde_json;

/**
 * AboutUserQueryResultView
 * This struct represents the result view for the "about user" query.
 * It contains the user's first name, last name, email, phone number, and status.
 * It implements the QueryResultView trait to provide a JSON representation of the data.
 */
#[derive(Serialize, Deserialize)]
pub struct AboutUserQueryResultView {
    first_name: String,
    last_name: String,
    email: String,
    phone: String,
    status: String,
}

impl AboutUserQueryResultView {
    /**
     * Creates a new instance of AboutUserQueryResultView.
     *
     * # Arguments
     * * `first_name` - The first name of the user.
     * * `last_name` - The last name of the user.
     * * `email` - The email address of the user.
     * * `phone` - The phone number of the user.
     * * `status` - The status of the user (e.g., active, inactive).
     *
     * # Returns
     * A new instance of AboutUserQueryResultView.
     */
    pub fn new(first_name: &str, last_name: &str, email: &str, phone: &str, status: &str) -> Self {
        Self {
            first_name: first_name.to_string(),
            last_name: last_name.to_string(),
            email: email.to_string(),
            phone: phone.to_string(),
            status: status.to_string(),
        }
    }
}

impl QueryResultView for AboutUserQueryResultView {
    fn get_result(&self) -> QueryResult {
        let json = serde_json::to_value(&self).unwrap();
        QueryResult::JSON(json)
    }
}
