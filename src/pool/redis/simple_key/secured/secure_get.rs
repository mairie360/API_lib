use crate::redis::simple_key::secure_get_key;

pub async fn handle_secure_get(
    mut conn: deadpool_redis::Connection,
    key: &str,
) -> Result<String, anyhow::Error> {
    match secure_get_key(&mut conn, key).await {
        Ok(value) => Ok(value),
        Err(e) => Err(anyhow::anyhow!(format!("{}", e))),
    }
}
