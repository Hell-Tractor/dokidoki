use std::sync::Arc;

use crate::state::AppState;

mod extractors;
mod middleware;
mod rest;
mod response;

pub use extractors::{AuthUser, ValidatedJson};

pub fn router(state: Arc<AppState>) -> axum::Router {
    axum::Router::new()
        .nest("/api/v1", rest::api(state.clone()))
        .with_state(state)
}