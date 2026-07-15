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
    #[validate(length(min = 1, max = 50), custom(function = "validate_llm_responses"))]
    responses: Vec<String>,
}

fn validate_llm_responses(responses: &[String]) -> Result<(), validator::ValidationError> {
    if responses.iter().any(|response| response.is_empty() || response.len() > 10_000) {
        return Err(validator::ValidationError::new("invalid_llm_response_item"));
    }
    Ok(())
}

async fn queue_llm_responses(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    ValidatedJson(body): ValidatedJson<QueueLlmRequest>,
) -> ApiResult<String> {
    let count = body.responses.len();
    state.llm.queue_responses(body.responses);
    tracing::info!(count, "dev llm responses queued");
    Ok(ApiResponse::ok("ok".to_owned()))
}
