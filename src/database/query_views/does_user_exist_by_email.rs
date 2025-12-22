use crate::database::db_interface::DatabaseQueryView;
use crate::database::QUERY;
use std::fmt::Display;

/**
 * This module defines a query view for checking if a user exists by their email.
 * It implements the DatabaseQueryView trait, which requires a method to get the SQL request
 * and a method to get the type of query.
 */
pub struct DoesUserExistByEmailQueryView {
    email: String,
    query: QUERY,
}

impl DoesUserExistByEmailQueryView {
    /**
     * Creates a new instance of DoesUserExistByEmailQueryView.
     * # Arguments
     * * `email` - The email address to check for existence in the database.
     * # Returns
     * A new instance of DoesUserExistByEmailQueryView.
     */
    pub fn new(email: String) -> Self {
        Self {
            email,
            query: QUERY::DoesUserExistByEmail,
        }
    }

    /**
     * Returns the email address associated with this query view.
     * # Returns
     * A reference to the email string.
     */
    pub fn get_email(&self) -> &String {
        &self.email
    }
}

impl DatabaseQueryView for DoesUserExistByEmailQueryView {
    fn get_request(&self) -> String {
        format!(
            "SELECT EXISTS(SELECT 1 FROM users WHERE email = '{}')",
            self.email
        )
    }

    fn get_query_type(&self) -> QUERY {
        self.query
    }
}

impl Display for DoesUserExistByEmailQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DoesUserExistByEmailQueryView: email = {}", self.email)
    }
}
