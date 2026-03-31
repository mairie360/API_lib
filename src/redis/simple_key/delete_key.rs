use deadpool_redis::redis::AsyncCommands;
use deadpool_redis::Connection;

pub async fn delete_key(conn: &mut Connection, key: &str) -> Result<(), redis::RedisError> {
    match conn.del::<&str, i32>(key).await {
        Ok(0) => Err(redis::RedisError::from((
            redis::ErrorKind::Io,
            "Key not found",
        ))),
        Ok(_) => Ok(()),
        Err(err) => Err(err),
    }
}
