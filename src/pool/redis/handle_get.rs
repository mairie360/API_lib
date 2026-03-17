use crate::pool::AppState;
use axum::{extract::State, http::StatusCode, response::IntoResponse};
use std::sync::Arc;

pub async fn handle_get(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.redis_pool.get().await {
        Ok(_) => (StatusCode::OK, "Connexion Redis réussie !").into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Échec connexion Redis").into_response(),
    }
}
