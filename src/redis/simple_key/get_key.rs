use deadpool_redis::redis::AsyncCommands;
use deadpool_redis::Connection;

pub async fn get_key(conn: &mut Connection, key: &str) -> Result<String, redis::RedisError> {
    conn.get::<&str, String>(key).await
}
