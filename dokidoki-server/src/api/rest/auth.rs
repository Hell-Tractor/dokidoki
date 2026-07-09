use std::sync::Arc;

use axum::{extract::State, routing::post};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    api::{response::ApiResult, ValidatedJson},
    state::AppState,
};

pub fn api() -> axum::Router<Arc<AppState>> {
    axum::Router::new()
        .route("/register", post(register))
}

#[derive(Deserialize, Validate)]
struct RegisterRequest {
    username: String,
    #[validate(length(min = 8))]
    password: String,
    display_name: String,
    birthday: Option<NaiveDate>,
}

#[derive(Serialize)]
struct UserResponse {
    id: String,
    username: String,
    display_name: String,
    birthday: NaiveDate,
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
    todo!()
}