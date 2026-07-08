use std::sync::Arc;

use crate::state::AppState;

pub fn api() -> axum::Router<Arc<AppState>> {
    axum::Router::new()
}