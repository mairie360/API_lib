use super::utils::QueryResult;
use crate::database::db_interface::QueryResultView;

/**
 * Represents the result of a query that checks if a user exists by their ID.
 */
pub struct DoesUserExistByIdQueryResultView {
    does_user_exist: bool,
}

/**
 * A view for the result of a query that checks if a user exists by their ID.
 */
impl DoesUserExistByIdQueryResultView {
    /**
     * Creates a new instance of `DoesUserExistByIdQueryResultView`.
     *
     * # Arguments
     *
     * * `does_user_exist` - A boolean indicating whether the user exists.
     */
    pub fn new(does_user_exist: bool) -> Self {
        Self { does_user_exist }
    }
}

impl QueryResultView for DoesUserExistByIdQueryResultView {
    fn get_result(&self) -> QueryResult {
        QueryResult::Boolean(self.does_user_exist)
    }
}
