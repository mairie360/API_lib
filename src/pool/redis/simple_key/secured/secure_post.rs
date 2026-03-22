use crate::redis::simple_key::secure_add_key;

pub async fn handle_secure_post(
    mut conn: deadpool_redis::Connection,
    key: &str,
    value: &str,
) -> Result<(), anyhow::Error> {
    match secure_add_key(&mut conn, key, value).await {
        Ok(_) => Ok(()),
        Err(e) => Err(anyhow::anyhow!(format!("{}", e))),
    }
}
