use crate::redis::simple_key::key_exist;

pub async fn handle_check_key(
    mut conn: deadpool_redis::Connection,
    key: &str,
) -> Result<(), anyhow::Error> {
    match key_exist(&mut conn, key).await {
        Ok(true) => Ok(()),
        Ok(false) => Err(anyhow::anyhow!("Key does not exist")),
        Err(e) => Err(anyhow::anyhow!(format!("{}", e))),
    }
}
