use super::utils::QueryResult;
use crate::database::db_interface::QueryResultView;

/**
 * View for the result of a login user query.
 * This view is used to return the number of users logged in.
 * It implements the QueryResultView trait.
 * It contains a single field `user_id` which is the ID of the user that has logged in.
 */
pub struct LoginUserQueryResultView {
    user_id: u64,
}

impl LoginUserQueryResultView {
    /**
     * Creates a new instance of `LoginUserQueryResultView`.
     *
     * # Arguments
     *
     * * `user_id` - The ID of the user that has logged in.
     */
    pub fn new(user_id: u64) -> Self {
        Self { user_id }
    }
}

impl QueryResultView for LoginUserQueryResultView {
    fn get_result(&self) -> QueryResult {
        QueryResult::U64(self.user_id)
    }
}
