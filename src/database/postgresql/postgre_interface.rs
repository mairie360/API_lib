use crate::env_manager::get_critical_env_var;
use super::queries::*;
use crate::database::db_interface::{DatabaseInterfaceActions, DatabaseQueryView, QueryResultView};
use crate::database::queries::QUERY::*;

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, LazyLock};
use tokio::sync::Mutex;
use tokio_postgres::{Client, NoTls};

static POSTGRESQL_INTERFACE: LazyLock<Mutex<Option<PostgreInterface>>> =
    LazyLock::new(|| Mutex::new(Some(PostgreInterface::new())));

/**
 * This function initializes the PostgreSQL interface if it hasn't been created yet.
 * It uses a mutex to ensure that only one instance is created in a thread-safe manner.
 * The PostgreInterface struct holds the database connection details and a client for executing queries.
 */
pub async fn create_postgre_interface() {
    let mut guard = POSTGRESQL_INTERFACE.lock().await;
    if guard.is_none() {
        *guard = Some(PostgreInterface::new());
    }
}

/**
 * This function retrieves the PostgreSQL interface, ensuring that it is initialized.
 * It returns a guard to the mutex that protects the interface, allowing safe access to it.
 * The interface can be used to connect to the database, execute queries, and manage the connection.
 * # Returns
 * A `MutexGuard` that provides access to the `PostgreInterface`.
 */
pub async fn get_postgre_interface() -> tokio::sync::MutexGuard<'static, Option<PostgreInterface>> {
    POSTGRESQL_INTERFACE.lock().await
}

/**
 * The PostgreInterface struct represents the interface for interacting with a PostgreSQL database.
 * It contains the database connection details and a client for executing queries.
 * The struct implements the DatabaseInterfaceActions trait, which defines methods for connecting to the database,
 * disconnecting, and executing queries.
 */
pub struct PostgreInterface {
    db_name: String,
    db_user: String,
    db_password: String,
    db_host: String,
    client: Arc<Mutex<Option<Client>>>,
}

impl PostgreInterface {
    /**
     * Creates a new instance of the PostgreInterface.
     * It retrieves the database connection details from environment variables,
     * ensuring that they are set before proceeding.
     * # Returns
     * A new instance of PostgreInterface with the database connection details.
     */
    pub fn new() -> Self {
        PostgreInterface {
            db_name: get_critical_env_var("DB_NAME"),
            db_user: get_critical_env_var("DB_USER"),
            db_password: get_critical_env_var("DB_PASSWORD"),
            db_host: get_critical_env_var("DB_HOST"),
            client: Arc::new(Mutex::new(None)),
        }
    }

    /**
     * Retrieves the client used for executing queries.
     * This method returns an Arc<Mutex<Option<Client>>> that allows safe access to the client.
     * The client is wrapped in a Mutex to ensure thread safety when accessing it.
     * # Returns
     * An Arc<Mutex<Option<Client>>> that provides access to the PostgreSQL client.
     */
    pub fn get_client(&self) -> Arc<Mutex<Option<Client>>> {
        self.client.clone()
    }
}

impl DatabaseInterfaceActions for PostgreInterface {
    fn connect(&mut self) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>> {
        let client_ref = self.client.clone();
        let config = format!(
            "host={} user={} password={} dbname={}",
            self.db_host, self.db_user, self.db_password, self.db_name
        );

        Box::pin(async move {
            let (client, connection) = tokio_postgres::connect(config.as_str(), NoTls)
                .await
                .map_err(|e| format!("Failed to connect: {}", e))?;

            // Spawn the connection future
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    eprintln!("Connection error: {}", e);
                }
            });

            *client_ref.lock().await = Some(client);

            Ok("PostgreSQL Connected".to_string())
        })
    }

    fn disconnect(&mut self) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>> {
        Box::pin(async move { Ok(String::from("PostgreSql Disconnected")) })
    }

    fn execute_query(
        &self,
        query: Box<dyn DatabaseQueryView>,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn QueryResultView>, String>> + Send>> {
        let client = self.get_client();
        Box::pin(async move {
            match query.get_query_type() {
                AboutUser => about_user(query, client).await,
                DoesUserExistByEmail => does_user_exist_by_email(query, client).await,
                DoesUserExistById => does_user_exist_by_id(query, client).await,
                RegisterUser => register_user(query, client).await,
                LoginUser => login_user(query, client).await,
                UnknownQuery => Err(format!(
                    "Unsupported query type: {}",
                    query.get_query_type()
                )),
            }
        })
    }
}
