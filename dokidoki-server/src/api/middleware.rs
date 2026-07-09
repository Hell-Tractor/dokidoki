use std::sync::Arc;

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};

use crate::{
    api::extractors::AuthContext,
    auth,
    error::AppError,
    state::AppState,
};

pub async fn require_auth(
    State(state): State<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = bearer_token(&req)?;
    let user = auth::authenticate(&state.db, token).await?;
    req.extensions_mut().insert(AuthContext { user });
    Ok(next.run(req).await)
}

fn bearer_token(req: &Request) -> Result<&str, AppError> {
    let header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .ok_or_else(AppError::invalid_token)?;
    let header = header
        .to_str()
        .map_err(|_| AppError::invalid_token())?;
    header
        .strip_prefix("Bearer ")
        .ok_or_else(AppError::invalid_token)
}
