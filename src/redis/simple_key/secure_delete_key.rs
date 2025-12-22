use super::delete_key;
use super::key_exist;
use redis::Connection;

/**
 * Securely deletes a key from Redis if it exists.
 * This function first checks if the key exists in the Redis database.
 * If the key exists, it proceeds to delete it. If the key does not exist,
 *
 * # Arguments
 * * `conn` - A mutable reference to the Redis connection.
 * * `key` - The key to be securely deleted from Redis.
 *
 * # Returns
 * * `Ok(())` if the key was successfully deleted.
 * * `Err(redis::RedisError)` if the key does not exist or if there was an error during the operation.
 */
pub async fn secure_delete_key(conn: &mut Connection, key: &str) -> Result<(), redis::RedisError> {
    match key_exist(conn, key).await {
        Ok(true) => delete_key(conn, key).await,
        Ok(false) => Err(redis::RedisError::from((
            redis::ErrorKind::ResponseError,
            "Key does not exist",
            format!("Key '{}' does not exist", key),
        ))),
        Err(err) => Err(err),
    }
}
