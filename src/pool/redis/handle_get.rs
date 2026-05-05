use crate::pool::AppState;
use axum::{extract::State, http::StatusCode, response::IntoResponse};
use std::sync::Arc;

pub async fn handle_get(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let pool = match state.redis_pool.as_ref() {
        Some(pool) => pool,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Redis pool not initialized",
            )
                .into_response()
        }
    };
    match pool.get().await {
        Ok(_) => (StatusCode::OK, "Connexion Redis réussie !").into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Échec connexion Redis").into_response(),
    }
}
