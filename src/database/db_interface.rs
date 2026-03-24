use crate::database::errors::DatabaseError;

/**
 * QueryResultView Trait
 * This trait defines the behavior of a query result view.
 * It provides a method to get the result of a query.
 */
pub trait QueryResultView {
    type Output;

    /// Returns the result of the query as a QueryResult.
    /// This method should be implemented by any struct that implements this trait.
    fn get_result(&self) -> Result<Self::Output, DatabaseError>;
}

/**
 * DatabaseQueryView Trait
 * This trait defines the behavior of a database query view.
 * It provides methods to get the request string and the type of query.
 */
pub trait DatabaseQueryView: Send {
    /**
     * Returns the request string of the query.
     * This method should be implemented by any struct that implements this trait.
     * It is expected to return a string representation of the query request.
     */
    fn get_request(&self) -> String;
}
