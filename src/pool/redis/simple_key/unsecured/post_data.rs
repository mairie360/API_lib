use crate::pool::AppState;
use crate::redis::simple_key::add_key;
use axum::extract::State;
use std::sync::Arc;

pub async fn handle_post_data(
    State(state): State<Arc<AppState>>,
    key: &str,
    value: &str,
) -> Result<(), anyhow::Error> {
    let mut conn = match state.redis_pool.get().await {
        Ok(c) => c,
        Err(_) => return Err(anyhow::anyhow!("Redis Pool Error")),
    };

    match add_key(&mut conn, key, value).await {
        Ok(_) => Ok(()),
        Err(e) => Err(anyhow::anyhow!(format!("{}", e)))
    }
}
