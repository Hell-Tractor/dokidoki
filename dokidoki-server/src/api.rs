use std::sync::Arc;

use crate::state::AppState;

mod extractors;
mod middleware;
mod rest;
mod response;
mod ws;

pub use extractors::{AuthUser, ValidatedJson, ValidatedQuery};

pub fn router(state: Arc<AppState>) -> axum::Router {
    axum::Router::new()
        .nest("/api/v1", rest::api(state.clone()))
        .with_state(state)
}