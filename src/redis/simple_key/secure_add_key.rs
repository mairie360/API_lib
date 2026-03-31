use super::add_key;
use super::key_exist;
use deadpool_redis::Connection;

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
