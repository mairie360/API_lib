use redis::{Commands, Connection};

/**
 * Deletes a key from Redis.
 *
 * # Arguments
 * * `conn` - A mutable reference to the Redis connection.
 * * `key` - The key to delete.
 *
 * # Returns
 * * `Ok(())` if the key was successfully deleted.
 * * `Err(redis::RedisError)` if there was an error during the operation or if the key was not found.
 */
pub async fn delete_key(conn: &mut Connection, key: &str) -> Result<(), redis::RedisError> {
    match conn.del(key) {
        Ok(0) => Err(redis::RedisError::from((
            redis::ErrorKind::Io,
            "Key not found",
        ))),
        Ok(_) => Ok(()),
        Err(err) => Err(err),
    }
}
