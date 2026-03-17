use crate::pool::AppState;
use crate::redis::simple_key::set_key;
use axum::extract::State;
use std::sync::Arc;

pub async fn handle_update_data(State(state): State<Arc<AppState>>, key: &str, value: &str) -> Result<(), anyhow::Error> {
    // Acquisition de la connexion au pool
    let mut conn = match state.redis_pool.get().await {
        Ok(c) => c,
        Err(_) => return Err(anyhow::anyhow!("Redis Pool Error")),
    };

    // Appel de set_key (écrase la valeur précédente)
    match set_key(&mut conn, key, value).await {
        Ok(_) => Ok(()),
        Err(e) => Err(anyhow::anyhow!(format!("{}", e))),
    }
}
