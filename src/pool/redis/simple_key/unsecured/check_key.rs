use crate::pool::AppState;
use crate::redis::simple_key::key_exist;
use axum::extract::State;
use std::sync::Arc;

pub async fn handle_check_key(
    State(state): State<Arc<AppState>>,
    key: &str,
) -> Result<(), anyhow::Error> {
    let mut conn = match state.redis_pool.get().await {
        Ok(c) => c,
        Err(_) => return Err(anyhow::anyhow!("Redis Pool Error")),
    };

    match key_exist(&mut conn, key).await {
        Ok(true) => Ok(()),
        Ok(false) => Err(anyhow::anyhow!("Key does not exist")),
        Err(e) => Err(anyhow::anyhow!(format!("{}", e)))
    }
}
