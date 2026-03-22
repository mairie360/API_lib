use crate::redis::simple_key::secure_delete_key;

pub async fn handle_secure_delete(
    mut conn: deadpool_redis::Connection,
    key: &str,
) -> Result<(), anyhow::Error> {
    match secure_delete_key(&mut conn, key).await {
        Ok(_) => Ok(()),
        Err(e) => Err(anyhow::anyhow!(format!("{}", e))),
    }
}
