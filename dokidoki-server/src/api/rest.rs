use std::sync::Arc;

use crate::state::AppState;

mod auth;
mod health;

pub fn api() -> axum::Router<Arc<AppState>> {
    axum::Router::new()
        .merge(health::api())
        .nest("/auth", auth::api())
}