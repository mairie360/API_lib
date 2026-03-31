use deadpool_redis::redis::AsyncCommands;
use deadpool_redis::Connection;

pub async fn key_exist(conn: &mut Connection, key: &str) -> Result<bool, redis::RedisError> {
    conn.exists(key).await
}
