use deadpool_redis::redis::AsyncCommands;
use deadpool_redis::Connection;

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
