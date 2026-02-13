use redis::{Commands, Connection};

/**
 * Check if a key exists in Redis.
 *
 * # Arguments
 * * `conn` - A mutable reference to the Redis connection.
 * * `key` - The key to check for existence.
 *
 * # Returns
 * A `Result` that is `Ok(true)` if the key exists, `Ok(false)` if it does not,
 * or an `Err` if there was an error checking the key's existence.
 */
pub async fn key_exist(conn: &mut Connection, key: &str) -> Result<bool, redis::RedisError> {
    match conn.exists(key) {
        Ok(exists) => Ok(exists),
        Err(err) => Err(err),
    }
}
