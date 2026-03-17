use super::add_key;
use super::key_exist;
use deadpool_redis::Connection;

/**
 * Securely adds a key to Redis if it does not already exist.
 * If the key already exists, it returns an error.
 *
 * # Arguments
 * * `conn` - A mutable reference to the Redis connection.
 * * `key` - The key to be added.
 * * `value` - The value to be associated with the key.
 *
 * # Returns
 * * `Ok(())` if the key was added successfully.
 * * `Err(redis::RedisError)` if the key already exists or if there was an error.
 */
pub async fn secure_add_key(
    conn: &mut Connection,
    key: &str,
    value: &str,
) -> Result<(), redis::RedisError> {
    if key_exist(conn, key).await? {
        return Err(redis::RedisError::from((
            redis::ErrorKind::Io,
            "Key already exists",
            format!("Key '{}' already exists", key),
        )));
    }
    add_key(conn, key, value).await
}
