use super::postgresql::postgre_interface::{create_postgre_interface, get_postgre_interface};
use crate::database::queries_result_views::QueryResult;
use crate::database::QUERY;
use std::collections::HashMap;
use std::env;
use std::future::Future;
use std::pin::Pin;
use std::sync::{LazyLock, Mutex};

/**
 * Database Interface Module
 */
static DB_INTERACE: LazyLock<Mutex<Option<DbInterface>>> =
    LazyLock::new(|| Mutex::new(Some(DbInterface::new())));

/**
 * Database Type Enum
 * This enum represents the different types of databases supported by the application.
 */
static DB_TYPES: LazyLock<HashMap<String, DatabaseType>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    map.insert(String::from("postgres"), DatabaseType::PostgreSQL);
    map
});

/**
 * Function to get the database type based on the key.
 * If the key is not found, it returns DatabaseType::Unknown.
 */
fn get_db_type(key: &str) -> DatabaseType {
    DB_TYPES.get(key).cloned().unwrap_or(DatabaseType::Unknown)
}

/**
 * Function to get a reference to the database interface.
 * This function returns a static reference to a Mutex-wrapped Option of DbInterface.
 */
pub fn get_db_interface() -> &'static Mutex<Option<DbInterface>> {
    &DB_INTERACE
}

/**
 * DatabaseType Enum
 * This enum represents the different types of databases supported by the application.
 * Currently, it supports PostgreSQL and an Unknown type for unsupported databases.
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DatabaseType {
    PostgreSQL,
    Unknown,
}

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
    /**
     * Returns the type of query.
     * This method should be implemented by any struct that implements this trait.
     * It is expected to return a QUERY enum value representing the type of query.
     */
    fn get_query_type(&self) -> QUERY;
}

pub trait DatabaseInterfaceActions: Send {
    /**
     * Connects to the database.
     * This method should be implemented by any struct that implements this trait.
     * It is expected to return a Future that resolves to a Result containing a success message or an error message.
     */
    fn connect(&mut self) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>>;
    /**
     * Disconnects from the database.
     * This method should be implemented by any struct that implements this trait.
     * It is expected to return a Future that resolves to a Result containing a success message or an error message.
     */
    fn disconnect(&mut self) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>>;
    /**
     * Executes a query on the database.
     * This method should be implemented by any struct that implements this trait.
     * It is expected to return a Future that resolves to a Result containing a QueryResultView or an error message.
     */
    fn execute_query(
        &self,
        query: Box<dyn DatabaseQueryView>,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn QueryResultView>, String>> + Send>>;
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
    pub fn new() -> Self {
        println!("Initializing database interface...");
        let db_type = env::var("DB_TYPE");
        println!("Database type: {:?}", db_type);
        match get_db_type(db_type.clone().unwrap().as_str()) {
            DatabaseType::PostgreSQL => {
                create_postgre_interface();
            }
            _ => {
                eprintln!("Unsupported database type: {}", db_type.unwrap().as_str());
                std::process::exit(1);
            }
        }
        DbInterface {}
    }

    /**
     * Connects to the database.
     */
    pub async fn connect(&mut self) -> Result<String, String> {
        let mut guard = get_postgre_interface().await;
        if let Some(ref mut postgre_interface) = *guard {
            match postgre_interface.connect().await {
                Ok(message) => Ok(message),
                Err(e) => Err(e),
            }
        } else {
            Err("PostgreInterface not initialized".to_string())
        }
    }

    /**
     * Disconnects from the database.
     */
    pub async fn disconnect(&mut self) -> Result<String, String> {
        let mut guard = get_postgre_interface().await;
        if let Some(ref mut postgre_interface) = *guard {
            match postgre_interface.disconnect().await {
                Ok(message) => Ok(message),
                Err(e) => Err(e),
            }
        } else {
            Err("PostgreInterface not initialized".to_string())
        }
    }

    /**
     * Executes a query on the database.
     * This method takes a query as a parameter, executes it, and returns the result.
     * It returns a Result containing a QueryResultView or an error message.
     */
    pub async fn execute_query(
        &self,
        query: Box<dyn DatabaseQueryView>,
    ) -> Result<Box<dyn QueryResultView>, String> {
        let guard = get_postgre_interface().await;
        if let Some(ref postgre_interface) = *guard {
            match postgre_interface.execute_query(query).await {
                Ok(result) => Ok(result),
                Err(e) => Err(e),
            }
        } else {
            Err("PostgreInterface not initialized".to_string())
        }
    }
}
