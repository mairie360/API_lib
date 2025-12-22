use crate::database::db_interface::DatabaseQueryView;
use crate::database::QUERY;
use std::fmt::Display;

/**
 * Query view to check if a user exists by their ID.
 */
pub struct DoesUserExistByIdQueryView {
    id: u64,
    query: QUERY,
}

impl DoesUserExistByIdQueryView {
    /**
     * Creates a new instance of `DoesUserExistByIdQueryView`.
     *
     * # Arguments
     * * `id` - The ID of the user to check for existence.
     *
     * # Returns
     * A new instance of `DoesUserExistByIdQueryView`.
     */
    pub fn new(id: u64) -> Self {
        Self {
            id,
            query: QUERY::DoesUserExistById,
        }
    }

    /**
     * Returns the ID of the user being checked.
     *
     * # Returns
     * A reference to the user's ID.
     */
    pub fn get_id(&self) -> &u64 {
        &self.id
    }
}

impl DatabaseQueryView for DoesUserExistByIdQueryView {
    fn get_request(&self) -> String {
        format!(
            "SELECT EXISTS(SELECT 1 FROM users WHERE id = '{}')",
            self.id
        )
    }

    fn get_query_type(&self) -> QUERY {
        self.query
    }
}

impl Display for DoesUserExistByIdQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DoesUserExistByIdQueryView: id = {}", self.id)
    }
}
