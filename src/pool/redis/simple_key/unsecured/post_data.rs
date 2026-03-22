use crate::redis::simple_key::add_key;

pub async fn handle_post_data(
    mut conn: deadpool_redis::Connection,
    key: &str,
    value: &str,
) -> Result<(), anyhow::Error> {
    match add_key(&mut conn, key, value).await {
        Ok(_) => Ok(()),
        Err(e) => Err(anyhow::anyhow!(format!("{}", e))),
    }
}
