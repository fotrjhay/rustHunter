pub mod apply;
pub mod search;

use axum::{
    routing::{get, post},
    Json, Router,
};

use crate::models::HealthResponse;
use crate::AppState;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/search", get(search::search_jobs))
        .route("/apply", post(apply::apply_to_vacancy))
        .with_state(state)
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}
