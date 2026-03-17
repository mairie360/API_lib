use deadpool_redis::redis::AsyncCommands;
use deadpool_redis::Connection;

/**
 * Sets a key in Redis with the given value.
 *
 * # Arguments
 * * `conn` - A mutable reference to the Redis connection.
 * * `key` - The key to set in Redis.
 * * `value` - The value to associate with the key.
 *
 * # Returns
 * `Result<(), redis::RedisError>` - Returns `Ok(())` if the operation was successful,
 * otherwise returns an error.
 */
pub async fn set_key(
    conn: &mut Connection,
    key: &str,
    value: &str,
) -> Result<(), redis::RedisError> {
    let response: String = conn.set(key, value).await?;
    if response == "OK" {
        Ok(())
    } else {
        Err(redis::RedisError::from((
            redis::ErrorKind::Io,
            "Unexpected SET response",
        )))
    }
}
