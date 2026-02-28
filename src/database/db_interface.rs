use super::postgresql::postgre_interface::{create_postgre_interface, get_postgre_interface};
use crate::database::errors::DatabaseError;
use crate::database::queries_result_views::utils::QueryResult;
use crate::database::QUERY;
use async_trait::async_trait;
use std::error::Error;
use std::sync::{LazyLock, Mutex};
use tokio_postgres::Client;

/**
 * Database Interface Module
 */
static DB_INTERFACE: LazyLock<Mutex<Option<DbInterface>>> = LazyLock::new(|| Mutex::new(None));

pub async fn init_db_interface() {
    let db_interface = DbInterface::new().await;
    let mut guard = DB_INTERFACE.lock().unwrap();
    *guard = Some(db_interface);
}

/**
 * Function to get a reference to the database interface.
 * This function returns a static reference to a Mutex-wrapped Option of DbInterface.
 */
pub fn get_db_interface() -> &'static Mutex<Option<DbInterface>> {
    &DB_INTERFACE
}

//                      -- Query --

/**
 * QueryResultView Trait
 * This trait defines the behavior of a query result view.
 * It provides a method to get the result of a query.
 */
pub trait QueryResultView {
    /// Returns the result of the query as a QueryResult.
    /// This method should be implemented by any struct that implements this trait.
    fn get_result(&self) -> QueryResult;
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
    fn get_raw_request(&self) -> String;
    /**
     * Returns the type of query.
     * This method should be implemented by any struct that implements this trait.
     * It is expected to return a QUERY enum value representing the type of query.
     */
    fn get_query_type(&self) -> QUERY;
}

#[async_trait]
pub trait Query {
    type Output: QueryResultView;
    type Error: Error + Send + Sync + 'static;

    async fn execute(&self, client: &Client) -> Result<Self::Output, Self::Error>;
}

//                      -- Database Interface --

#[async_trait]
pub trait DatabaseInterfaceActions: Send {
    /**
     * Connects to the database.
     * This method should be implemented by any struct that implements this trait.
     * It is expected to return a Future that resolves to a Result containing a success message or an error message.
     */
    async fn connect(&mut self) -> Result<String, DatabaseError>;
    /**
     * Disconnects from the database.
     * This method should be implemented by any struct that implements this trait.
     * It is expected to return a Result containing a success message or an error message.
     */
    async fn disconnect(&mut self) -> Result<String, DatabaseError>;
    /**
     * Executes a query on the database.
     * This method should be implemented by any struct that implements this trait.
     * It is expected to return a Future that resolves to a Result containing a QueryResultView or an error message.
     */
    async fn execute_query<Q>(&self, query: Q) -> Result<Q::Output, Box<dyn Error + Send + Sync>>
    where
        Q: Query + Send + 'static;
}

/**
 * DbInterface Struct
 * This struct represents the database interface.
 * It is responsible for managing the connection to the database and executing queries.
 */
pub struct DbInterface {
    // db_interface: Box<dyn DatabaseInterfaceActions + Send>
}

impl DbInterface {
    /**
     * Creates a new instance of DbInterface.
     * This method initializes the database interface based on the environment variable DB_TYPE.
     * If the DB_TYPE is not supported, it will print an error message and exit the program.
     */
    pub async fn new() -> Self {
        create_postgre_interface().await;
        DbInterface {}
    }

    /**
     * Connects to the database.
     */
    pub async fn connect(&mut self) -> Result<String, DatabaseError> {
        let mut guard = get_postgre_interface().await;
        if let Some(ref mut postgre_interface) = *guard {
            match postgre_interface.connect().await {
                Ok(message) => Ok(message),
                Err(e) => Err(e),
            }
        } else {
            Err(DatabaseError::Internal(
                "PostgreInterface not initialized".to_string(),
            ))
        }
    }

    /**
     * Disconnects from the database.
     */
    pub async fn disconnect(&mut self) -> Result<String, DatabaseError> {
        let mut guard = get_postgre_interface().await;
        if let Some(ref mut postgre_interface) = *guard {
            match postgre_interface.disconnect().await {
                Ok(message) => Ok(message),
                Err(e) => Err(e),
            }
        } else {
            Err(DatabaseError::Internal(
                "PostgreInterface not initialized".to_string(),
            ))
        }
    }

    /**
     * Executes a query on the database.
     * This method takes a query as a parameter, executes it, and returns the result.
     * It returns a Result containing a QueryResultView or an error message.
     */
    pub async fn execute_query<Q>(
        &self,
        query: Q,
    ) -> Result<Q::Output, Box<dyn Error + Send + Sync>>
    where
        Q: Query + Send + 'static,
    {
        let guard = get_postgre_interface().await;
        if let Some(ref postgre_interface) = *guard {
            postgre_interface.execute_query(query).await
        } else {
            Err(Box::new(DatabaseError::Internal(
                "PostgreInterface not initialized".to_string(),
            )) as Box<dyn Error + Send + Sync>)
        }
    }
}
