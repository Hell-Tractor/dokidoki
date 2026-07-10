use std::sync::Arc;

use axum::middleware;

use crate::{api::middleware::require_auth, state::AppState};

mod auth;
mod characters;
mod conversations;
mod health;
mod users;

pub fn api(state: Arc<AppState>) -> axum::Router<Arc<AppState>> {
    let public = axum::Router::new()
        .merge(health::api())
        .nest("/auth", auth::api());

    let protected = axum::Router::new()
        .merge(users::api())
        .nest("/characters", characters::api())
        .nest("/conversations", conversations::api())
        .layer(middleware::from_fn_with_state(state, require_auth));

    public.merge(protected)
}
