use crate::database::db_interface::DatabaseQueryView;
use crate::database::QUERY;
use std::fmt::Display;

/**
 * LoginUserQueryView
 * This struct is used to represent a query for logging in a user.
 * It contains the user's email and password, and implements the DatabaseQueryView trait.
 * It is used to generate the SQL query for logging in a user.
 */
pub struct LoginUserQueryView {
    email: String,
    password: String,
    query: QUERY,
}

impl LoginUserQueryView {
    /**
     * Creates a new LoginUserQueryView instance.
     *
     * # Arguments
     *
     * * `email` - The email of the user.
     * * `password` - The password of the user.
     *
     * # Returns
     *
     * A new instance of LoginUserQueryView.
     */
    pub fn new(email: String, password: String) -> Self {
        Self {
            email,
            password,
            query: QUERY::LoginUser,
        }
    }

    /**
     * Returns the email of the user.
     * # Returns
     * A reference to the email string.
     */
    pub fn get_email(&self) -> &String {
        &self.email
    }

    /**
     * Returns the password of the user.
     * # Returns
     * A reference to the password string.
     */
    pub fn get_password(&self) -> &String {
        &self.password
    }
}

impl DatabaseQueryView for LoginUserQueryView {
    fn get_request(&self) -> String {
        format!(
            "SELECT id FROM users WHERE email = '{}' AND password = '{}'",
            self.email, self.password
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
            "LoginUserQueryView: email = {}, password = {}",
            self.email, self.password
        )
    }
}
