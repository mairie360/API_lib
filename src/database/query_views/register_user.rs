use crate::database::db_interface::DatabaseQueryView;
use crate::database::QUERY;
use std::fmt::Display;

/**
 * This struct represents a query view for registering a user in the database.
 * It contains the necessary fields for user registration and implements the DatabaseQueryView trait.
 */
pub struct RegisterUserQueryView {
    first_name: String,
    last_name: String,
    email: String,
    password: String,
    phone_number: Option<String>,

    query: QUERY,
}

impl RegisterUserQueryView {
    /**
     * Creates a new instance of RegisterUserQueryView with the provided user details.
     *
     * # Arguments
     * * `first_name` - The first name of the user.
     * * `last_name` - The last name of the user.
     * * `email` - The email address of the user.
     * * `password` - The password for the user account.
     * * `phone_number` - An optional phone number for the user.
     *
     * # Returns
     * A new instance of RegisterUserQueryView.
     */
    pub fn new(
        first_name: String,
        last_name: String,
        email: String,
        password: String,
        phone_number: Option<String>,
    ) -> Self {
        Self {
            first_name,
            last_name,
            email,
            password,
            phone_number,
            query: QUERY::RegisterUser,
        }
    }

    /**
     * Returns the first name of the user.
     * # Returns
     * A reference to the first name string.
     */
    pub fn get_first_name(&self) -> &String {
        &self.first_name
    }

    /**
     * Returns the last name of the user.
     * # Returns
     * A reference to the last name string.
     */
    pub fn get_last_name(&self) -> &String {
        &self.last_name
    }

    /**
     * Returns the email address of the user.
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

    /**
     * Returns the phone number of the user, if available.
     * # Returns
     * A reference to an optional phone number string.
     */
    pub fn get_phone_number(&self) -> &Option<String> {
        &self.phone_number
    }
}

impl DatabaseQueryView for RegisterUserQueryView {
    fn get_request(&self) -> String {
        match self.phone_number {
            Some(ref phone) => format!(
                "INSERT INTO users (first_name, last_name, email, password, phone_number) VALUES ('{}', '{}', '{}', '{}', '{}')",
                self.first_name, self.last_name, self.email, self.password, phone
            ),
            None => format!(
                "INSERT INTO users (first_name, last_name, email, password) VALUES ('{}', '{}', '{}', '{}')",
                self.first_name, self.last_name, self.email, self.password
            ),
        }
    }

    fn get_query_type(&self) -> QUERY {
        self.query
    }
}

impl Display for RegisterUserQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RegisterUserQueryView: email = {}", self.email)
    }
}
