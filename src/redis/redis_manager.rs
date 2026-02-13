use super::simple_key;
use crate::env_manager::get_critical_env_var;
use redis::{Client, Connection};
use std::sync::LazyLock;
use tokio::sync::Mutex;

static REDIS_INTERFACE: LazyLock<Mutex<Option<RedisManager>>> =
    LazyLock::new(|| Mutex::new(Some(RedisManager::new())));

/**
 * RedisManager is a singleton that manages the Redis connection.
 * It provides methods to connect to Redis and perform basic operations like set, get, delete keys
 */
pub async fn create_redis_manager() {
    let mut guard = REDIS_INTERFACE.lock().await;
    if guard.is_none() {
        *guard = Some(RedisManager::new());
    }
}

/**
 * get_redis_manager returns a guard to the RedisManager singleton.
 * It allows access to the RedisManager instance for performing operations.
 *
 * # Returns
 * A `MutexGuard` that provides access to the `RedisManager` instance.
 */
pub async fn get_redis_manager() -> tokio::sync::MutexGuard<'static, Option<RedisManager>> {
    REDIS_INTERFACE.lock().await
}

/**
 * RedisManager is a struct that manages the Redis connection.
 * It provides methods to connect to Redis and perform basic operations like set, get, delete keys
 */
pub struct RedisManager {
    client: Client,
    connection: Option<Connection>,
}

impl RedisManager {
    /**
     * Creates a new RedisManager instance.
     * It initializes the Redis client using the REDIS_URL environment variable.
     * If the environment variable is not set, it will panic.
     * # Returns
     * A new instance of `RedisManager`.
     * # Panics
     * If the REDIS_URL environment variable is not set or if the Redis client cannot be created.
     */
    pub fn new() -> Self {
        let redis_url = get_critical_env_var("REDIS_URL");
        println!("Connecting to Redis at: {}", redis_url);
        let client = Client::open(redis_url).expect("Failed to create Redis client");
        RedisManager {
            client,
            connection: None,
        }
    }

    /**
     * connect establishes a connection to the Redis server.
     * It initializes the connection field with a Redis connection.
     *
     * # Returns
     * A `Result` that indicates whether the connection was successful or not.
     * If successful, it returns a success message.
     * If it fails, it returns a `RedisError` with an appropriate error message.
     *
     * # Errors
     * If the connection cannot be established, it returns a `RedisError` with an IoError kind.
     */
    pub fn connect(&mut self) -> Result<String, redis::RedisError> {
        self.connection = Some(self.client.get_connection()?);
        match self.connection {
            Some(_) => Ok("Connected to Redis successfully".to_string()),
            None => Err(redis::RedisError::from((
                redis::ErrorKind::Io,
                "Failed to connect to Redis",
            ))),
        }
    }

    /**
     * set_key sets a key-value pair in Redis.
     * It uses the `simple_key` module to perform the operation.
     *
     * # Arguments
     * - `key`: A string slice that holds the key to be set.
     * - `value`: A string slice that holds the value to be set.
     *
     * # Returns
     * A `Result` that indicates whether the operation was successful or not.
     * If successful, it returns `Ok(())`.
     * If it fails, it returns a `RedisError` with an appropriate error message.
     */
    pub async fn set_key(&mut self, key: &str, value: &str) -> Result<(), redis::RedisError> {
        match &mut self.connection {
            Some(conn) => simple_key::set_key(conn, key, value).await,
            None => Err(redis::RedisError::from((
                redis::ErrorKind::Io,
                "No Redis connection established",
            ))),
        }
    }

    /**
     * add_key sets a key-value pair in Redis.
     * It uses the `simple_key` module to perform the operation.
     *
     * # Arguments
     * - `key`: A string slice that holds the key to be set.
     * - `value`: A string slice that holds the value to be set.
     *
     * # Returns
     * A `Result` that indicates whether the operation was successful or not.
     * If successful, it returns `Ok(())`.
     * If it fails, it returns a `RedisError` with an appropriate error message.
     */
    pub async fn add_key(&mut self, key: &str, value: &str) -> Result<(), redis::RedisError> {
        match &mut self.connection {
            Some(conn) => simple_key::add_key(conn, key, value).await,
            None => Err(redis::RedisError::from((
                redis::ErrorKind::Io,
                "No Redis connection established",
            ))),
        }
    }

    /**
     * secure_add_key sets a key-value pair in Redis with secure handling.
     * It uses the `simple_key` module to perform the operation.
     *
     * # Arguments
     * - `key`: A string slice that holds the key to be set.
     * - `value`: A string slice that holds the value to be set.
     *
     * # Returns
     * A `Result` that indicates whether the operation was successful or not.
     * If successful, it returns `Ok(())`.
     * If it fails, it returns a `RedisError` with an appropriate error message.
     */
    pub async fn secure_add_key(
        &mut self,
        key: &str,
        value: &str,
    ) -> Result<(), redis::RedisError> {
        match &mut self.connection {
            Some(conn) => simple_key::secure_add_key(conn, key, value).await,
            None => Err(redis::RedisError::from((
                redis::ErrorKind::Io,
                "No Redis connection established",
            ))),
        }
    }

    /**
     * get_key retrieves the value associated with a key from Redis.
     * It uses the `simple_key` module to perform the operation.
     *
     * # Arguments
     * - `key`: A string slice that holds the key to be retrieved.
     *
     * # Returns
     * A `Result` that contains the value associated with the key if successful,
     * or a `RedisError` if it fails.
     * If successful, it returns `Ok(value)` where `value` is a string.
     * If it fails, it returns a `RedisError` with an appropriate error message.
     */
    pub async fn get_key(&mut self, key: &str) -> Result<String, redis::RedisError> {
        match &mut self.connection {
            Some(conn) => simple_key::get_key(conn, key).await,
            None => Err(redis::RedisError::from((
                redis::ErrorKind::Io,
                "No Redis connection established",
            ))),
        }
    }

    /**
     * secure_get_key retrieves the value associated with a key from Redis with secure handling.
     * It uses the `simple_key` module to perform the operation.
     *
     * # Arguments
     * - `key`: A string slice that holds the key to be retrieved.
     *
     * # Returns
     * A `Result` that contains the value associated with the key if successful,
     * or a `RedisError` if it fails.
     * If successful, it returns `Ok(value)` where `value` is a string.
     * If it fails, it returns a `RedisError` with an appropriate error message.
     */
    pub async fn secure_get_key(&mut self, key: &str) -> Result<String, redis::RedisError> {
        match &mut self.connection {
            Some(conn) => simple_key::secure_get_key(conn, key).await,
            None => Err(redis::RedisError::from((
                redis::ErrorKind::Io,
                "No Redis connection established",
            ))),
        }
    }

    /**
     * delete_key removes a key from Redis.
     * It uses the `simple_key` module to perform the operation.
     *
     * # Arguments
     * - `key`: A string slice that holds the key to be deleted.
     *
     * # Returns
     * A `Result` that indicates whether the operation was successful or not.
     * If successful, it returns `Ok(())`.
     * If it fails, it returns a `RedisError` with an appropriate error message.
     */
    pub async fn delete_key(&mut self, key: &str) -> Result<(), redis::RedisError> {
        match &mut self.connection {
            Some(conn) => simple_key::delete_key(conn, key).await,
            None => Err(redis::RedisError::from((
                redis::ErrorKind::Io,
                "No Redis connection established",
            ))),
        }
    }

    /**
     * secure_delete_key removes a key from Redis with secure handling.
     * It uses the `simple_key` module to perform the operation.
     *
     * # Arguments
     * - `key`: A string slice that holds the key to be deleted.
     *
     * # Returns
     * A `Result` that indicates whether the operation was successful or not.
     * If successful, it returns `Ok(())`.
     * If it fails, it returns a `RedisError` with an appropriate error message.
     */
    pub async fn secure_delete_key(&mut self, key: &str) -> Result<(), redis::RedisError> {
        match &mut self.connection {
            Some(conn) => simple_key::secure_delete_key(conn, key).await,
            None => Err(redis::RedisError::from((
                redis::ErrorKind::Io,
                "No Redis connection established",
            ))),
        }
    }

    /**
     * key_exist checks if a key exists in Redis.
     * It uses the `simple_key` module to perform the operation.
     *
     * # Arguments
     * - `key`: A string slice that holds the key to be checked.
     *
     * # Returns
     * A `Result` that indicates whether the key exists or not.
     * If the key exists, it returns `Ok(true)`.
     * If the key does not exist, it returns `Ok(false)`.
     * If it fails, it returns a `RedisError` with an appropriate error message.
     */
    pub async fn key_exist(&mut self, key: &str) -> Result<bool, redis::RedisError> {
        match &mut self.connection {
            Some(conn) => simple_key::key_exist(conn, key).await,
            None => Err(redis::RedisError::from((
                redis::ErrorKind::Io,
                "No Redis connection established",
            ))),
        }
    }
}
