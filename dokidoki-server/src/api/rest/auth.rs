use std::sync::Arc;

use axum::{extract::State, routing::post};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    api::{response::ApiResponse, response::ApiResult, ValidatedJson},
    auth::{self, RegisterParams},
    state::AppState,
};

pub fn api() -> axum::Router<Arc<AppState>> {
    axum::Router::new().route("/register", post(register))
}

#[derive(Deserialize, Validate)]
struct RegisterRequest {
    #[validate(length(min = 1, max = 64))]
    username: String,
    #[validate(length(min = 8, max = 32))]
    password: String,
    #[validate(length(max = 64))]
    display_name: Option<String>,
    birthday: Option<NaiveDate>,
}

#[derive(Serialize)]
struct UserResponse {
    id: String,
    username: String,
    display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    birthday: Option<NaiveDate>,
    max_proactive_per_day: u32,
}

#[derive(Serialize)]
struct RegisterResponse {
    token: String,
    user: UserResponse,
}

async fn register(
    State(state): State<Arc<AppState>>,
    ValidatedJson(body): ValidatedJson<RegisterRequest>,
) -> ApiResult<RegisterResponse> {
    let session = auth::register(
        &state.db,
        &state.config.auth,
        RegisterParams {
            username: body.username,
            password: body.password,
            display_name: body.display_name,
            birthday: body.birthday,
        },
    )
    .await?;

    Ok(ApiResponse::created(RegisterResponse {
        token: session.token,
        user: UserResponse {
            id: session.user.id,
            username: session.user.username,
            display_name: session.user.display_name,
            birthday: session.user.birthday,
            max_proactive_per_day: session.user.max_proactive_per_day as u32,
        },
    }))
}
