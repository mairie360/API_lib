use crate::redis::simple_key::get_key;

pub async fn handle_get_data(
    mut conn: deadpool_redis::Connection,
    key: &str,
) -> Result<String, anyhow::Error> {
    match get_key(&mut conn, key).await {
        Ok(value) => Ok(value),
        Err(e) => Err(anyhow::anyhow!(format!("{}", e))),
    }
}
