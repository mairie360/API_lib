use deadpool_redis::redis::AsyncCommands;
use deadpool_redis::Connection;

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
