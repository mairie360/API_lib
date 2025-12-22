use super::get_key;
use super::key_exist;
use redis::Connection;

/**
 * Securely retrieves a key from Redis if it exists.
 *
 * This function first checks if the key exists in Redis.
 * If the key exists, it retrieves the value associated with the key.
 * If the key does not exist, it returns a Redis error indicating that the key does not exist.
 *
 * # Arguments
 * * `conn` - A mutable reference to the Redis connection.
 * * `key` - The key to retrieve from Redis.
 *
 * # Returns
 * * `Ok(String)` - The value associated with the key if it exists.
 * * `Err(redis::RedisError)` - An error if the key does not exist or if there is an issue with the Redis connection.
 */
pub async fn secure_get_key(conn: &mut Connection, key: &str) -> Result<String, redis::RedisError> {
    match key_exist(conn, key).await {
        Ok(true) => get_key(conn, key).await,
        Ok(false) => Err(redis::RedisError::from((
            redis::ErrorKind::TypeError,
            "Key does not exist",
            format!("Key '{}' does not exist", key),
        ))),
        Err(err) => Err(err),
    }
}
