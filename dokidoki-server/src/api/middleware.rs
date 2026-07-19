use std::sync::Arc;

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};

use crate::{
    api::extractors::AuthContext,
    domain::auth,
    error::AppError,
    state::AppState,
};

pub async fn require_auth(
    State(state): State<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = extract_token(&req)?;
    let user = auth::authenticate(&state.db, &token).await?;
    req.extensions_mut().insert(AuthContext { user });
    Ok(next.run(req).await)
}

/// Prefer `Authorization: Bearer …`. Browser WebSocket cannot set custom headers,
/// so Flutter Web falls back to `?token=`.
fn extract_token(req: &Request) -> Result<String, AppError> {
    if let Some(header) = req.headers().get(axum::http::header::AUTHORIZATION) {
        let header = header
            .to_str()
            .map_err(|_| AppError::invalid_token())?;
        return header
            .strip_prefix("Bearer ")
            .filter(|token| !token.is_empty())
            .map(str::to_owned)
            .ok_or_else(AppError::invalid_token);
    }

    token_from_query(req.uri().query()).ok_or_else(AppError::invalid_token)
}

fn token_from_query(query: Option<&str>) -> Option<String> {
    let query = query?;
    for pair in query.split('&') {
        let Some((key, value)) = pair.split_once('=') else {
            continue;
        };
        if key == "token" && !value.is_empty() {
            return Some(value.to_owned());
        }
    }
    None
}
