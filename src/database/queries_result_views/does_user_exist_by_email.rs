use super::utils::QueryResult;
use crate::database::db_interface::QueryResultView;

/**
 * This struct represents the result of a query that checks if a user exists by their email.
 * It implements the QueryResultView trait to provide a way to retrieve the result.
 */
pub struct DoesUserExistByEmailQueryResultView {
    does_user_exist: bool,
}

impl DoesUserExistByEmailQueryResultView {
    /**
     * Creates a new instance of DoesUserExistByEmailQueryResultView.
     * # Arguments
     * * `does_user_exist`: A boolean indicating whether the user exists or not.
     */
    pub fn new(does_user_exist: bool) -> Self {
        Self { does_user_exist }
    }
}

impl QueryResultView for DoesUserExistByEmailQueryResultView {
    fn get_result(&self) -> QueryResult {
        QueryResult::Boolean(self.does_user_exist)
    }
}
