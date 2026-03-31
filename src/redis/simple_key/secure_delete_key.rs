use super::delete_key;
use super::key_exist;
use deadpool_redis::Connection;

pub async fn secure_delete_key(conn: &mut Connection, key: &str) -> Result<(), redis::RedisError> {
    match key_exist(conn, key).await {
        Ok(true) => delete_key(conn, key).await,
        Ok(false) => Err(redis::RedisError::from((
            redis::ErrorKind::Io,
            "Key does not exist",
            format!("Key '{}' does not exist", key),
        ))),
        Err(err) => Err(err),
    }
}
