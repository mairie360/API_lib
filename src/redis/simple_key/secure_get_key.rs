use super::get_key;
use super::key_exist;
use deadpool_redis::Connection;

pub async fn secure_get_key(conn: &mut Connection, key: &str) -> Result<String, redis::RedisError> {
    match key_exist(conn, key).await {
        Ok(true) => get_key(conn, key).await,
        Ok(false) => Err(redis::RedisError::from((
            redis::ErrorKind::Io,
            "Key does not exist",
            format!("Key '{}' does not exist", key),
        ))),
        Err(err) => Err(err),
    }
}
