use redis::{Commands, Connection};

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
    match conn.set::<&str, &str, String>(key, value) {
        Ok(response) => {
            if response == "OK" {
                Ok(())
            } else {
                Err(redis::RedisError::from((
                    redis::ErrorKind::Io,
                    "Unexpected SET response",
                )))
            }
        }
        Err(err) => Err(err),
    }
}
