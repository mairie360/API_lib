use redis::{Commands, Connection};

/**
 * Retrieves the value associated with the given key from Redis.
 *
 * # Arguments
 * * `conn` - A mutable reference to the Redis connection.
 * * `key` - The key for which the value is to be retrieved.
 *
 * # Returns
 * * `Ok(String)` - The value associated with the key if it exists.
 * * `Err(redis::RedisError)` - An error if the key does not exist or if there is an issue with the Redis connection.
 */
pub async fn get_key(conn: &mut Connection, key: &str) -> Result<String, redis::RedisError> {
    match conn.get::<&str, String>(key) {
        Ok(value) => Ok(value),
        Err(err) => Err(err),
    }
}
