use std::sync::Arc;

use axum::{routing::post, Router};
use serde::Deserialize;
use validator::Validate;

use crate::{
    api::{response::ApiResponse, response::ApiResult, ValidatedJson},
    state::AppState,
};

pub fn api() -> Router<Arc<AppState>> {
    Router::new().route("/dev/llm/queue", post(queue_llm_responses))
}

#[derive(Deserialize, Validate)]
struct QueueLlmRequest {
    responses: Vec<String>,
}

async fn queue_llm_responses(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    ValidatedJson(body): ValidatedJson<QueueLlmRequest>,
) -> ApiResult<String> {
    state.llm.queue_responses(body.responses);
    Ok(ApiResponse::ok("ok".to_owned()))
}
