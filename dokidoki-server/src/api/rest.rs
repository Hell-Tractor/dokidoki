use std::sync::Arc;

use axum::middleware;

use crate::{api::middleware::require_auth, api::ws, state::AppState};

mod auth;
mod characters;
mod conversations;
mod health;
mod messages;
mod users;

pub fn api(state: Arc<AppState>) -> axum::Router<Arc<AppState>> {
    let public = axum::Router::new()
        .merge(health::api())
        .nest("/auth", auth::api())
        .merge(dev_api());

    let protected = axum::Router::new()
        .merge(users::api())
        .nest("/characters", characters::api())
        .nest("/conversations", conversations::api())
        .route("/ws", axum::routing::get(ws::handler))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth));

    public.merge(protected)
}

fn dev_api() -> axum::Router<Arc<AppState>> {
    #[cfg(debug_assertions)]
    {
        return dev::api();
    }
    #[cfg(not(debug_assertions))]
    {
        axum::Router::new()
    }
}

#[cfg(debug_assertions)]
mod dev;
