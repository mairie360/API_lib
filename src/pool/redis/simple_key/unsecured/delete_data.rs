use crate::redis::simple_key::delete_key;

pub async fn handle_delete_data(
    mut conn: deadpool_redis::Connection,
    key: &str,
) -> Result<(), anyhow::Error> {
    match delete_key(&mut conn, key).await {
        Ok(_) => Ok(()),
        Err(e) => Err(anyhow::anyhow!(format!("{}", e))),
    }
}
