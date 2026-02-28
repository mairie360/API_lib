use super::utils::QueryResult;
use crate::database::db_interface::QueryResultView;

/**
 * This module defines the result view for the RegisterUser query.
 * It encapsulates the result of the query execution, which can either be a success or an error.
 */
pub struct RegisterUserQueryResultView {
    success: Result<(), String>,
}

impl RegisterUserQueryResultView {
    /**
     * Creates a new instance of RegisterUserQueryResultView.
     * * # Arguments
     *  * `success`: A Result type indicating the success or failure of the user registration.
     */
    pub fn new(success: Result<(), String>) -> Self {
        Self { success }
    }
}

impl QueryResultView for RegisterUserQueryResultView {
    fn get_result(&self) -> QueryResult {
        QueryResult::Result(self.success.clone())
    }
}
