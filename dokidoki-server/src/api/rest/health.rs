use std::sync::Arc;

use axum::routing::get;

use crate::{api::response::ApiResponse, state::AppState};

pub fn api() -> axum::Router<Arc<AppState>> {
    axum::Router::new().route("/health", get(health))
}

async fn health() -> ApiResponse<&'static str> {
    ApiResponse::ok_empty()
}
