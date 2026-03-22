use deadpool_redis::redis::AsyncCommands;
use deadpool_redis::Connection;

/**
 * Adds a key to Redis if it does not already exist.
 * This function attempts to set a key with a value in Redis using the `SETNX` command,
 *   which sets the key only if it does not already exist.
 *
 * # Arguments
 * * `conn` - A mutable reference to a Redis connection.
 * * `key` - The key to be added.
 * * `value` - The value to be associated with the key.
 *
 * # Returns
 * * `Ok(())` if the key was successfully added.
 * * `Err(redis::RedisError)` if the key already exists or if there was an error during the operation.
 */
pub async fn add_key(
    conn: &mut Connection,
    key: &str,
    value: &str,
) -> Result<(), redis::RedisError> {
    // Note l'utilisation de .set_nx().await
    match conn.set_nx::<&str, &str, bool>(key, value).await {
        Ok(true) => Ok(()),
        Ok(false) => Err(redis::RedisError::from((
            redis::ErrorKind::Io,
            "Key already exists",
        ))),
        Err(err) => Err(err),
    }
}
