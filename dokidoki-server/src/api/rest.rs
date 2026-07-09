use std::sync::Arc;

use crate::state::AppState;

mod auth;

pub fn api() -> axum::Router<Arc<AppState>> {
    axum::Router::new()
        .nest("/auth", auth::api())
}