use crate::pool::AppState;
use crate::redis::simple_key::get_key;
use axum::extract::State;
use std::sync::Arc;

pub async fn handle_get_data(
    State(state): State<Arc<AppState>>,
    key: &str,
) -> Result<String, anyhow::Error> {
    let mut conn = match state.redis_pool.get().await {
        Ok(c) => c,
        Err(_) => return Err(anyhow::anyhow!("Redis Pool Error")),
    };

    match get_key(&mut conn, key).await {
        Ok(value) => Ok(value),
        Err(e) => Err(anyhow::anyhow!(format!("{}", e)))
    }
}
