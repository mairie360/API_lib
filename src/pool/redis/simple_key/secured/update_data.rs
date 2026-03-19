use crate::redis::simple_key::set_key;

pub async fn handle_update_data(
    mut conn: deadpool_redis::Connection,
    key: &str,
    value: &str,
) -> Result<(), anyhow::Error> {
    match set_key(&mut conn, key, value).await {
        Ok(_) => Ok(()),
        Err(e) => Err(anyhow::anyhow!(format!("{}", e))),
    }
}
