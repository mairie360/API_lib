use crate::database::db_interface::DatabaseQueryView;
use crate::database::QUERY;
use std::fmt::Display;

/**
 * Query view for retrieving information about a user.
 * This view is used to fetch details such as first name, last name, email, phone number, and status of a user by their ID.
 */
pub struct AboutUserQueryView {
    id: u64,
    query: QUERY,
}

impl AboutUserQueryView {
    /**
     * Creates a new instance of `AboutUserQueryView`.
     *
     * # Arguments
     * `id` - The unique identifier of the user for whom the information is to be retrieved.
     *
     * # Returns
     * A new instance of `AboutUserQueryView` initialized with the provided user ID and
     */
    pub fn new(id: u64) -> Self {
        Self {
            id,
            query: QUERY::AboutUser,
        }
    }

    /**
     * Returns the user ID associated with this query view.
     *
     * # Returns
     * A reference to the user ID as a `u64`.
     */
    pub fn get_id(&self) -> &u64 {
        &self.id
    }
}

impl DatabaseQueryView for AboutUserQueryView {
    fn get_request(&self) -> String {
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
