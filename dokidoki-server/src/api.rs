use std::sync::Arc;

use axum::http::{Method, header};
use tower_http::cors::{AllowOrigin, CorsLayer};

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
        .layer(cors_layer())
        .with_state(state)
}

/// Flutter Web (Chrome) runs on a different origin than the API host, so browsers
/// require CORS. Native iOS/Android ignore these headers.
fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(AllowOrigin::mirror_request())
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ACCEPT,
        ])
        .allow_credentials(true)
        .expose_headers([header::CONTENT_TYPE])
        .max_age(std::time::Duration::from_secs(600))
}