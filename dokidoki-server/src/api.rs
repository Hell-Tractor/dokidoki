use std::sync::Arc;

use crate::state::AppState;

mod extractors;
mod rest;
mod response;

pub use extractors::ValidatedJson;

pub fn router(state: Arc<AppState>) -> axum::Router
{
    axum::Router::new()
        .nest("/api/v1", rest::api())
        .with_state(state)
}