use std::sync::Arc;

use axum::{extract::State, routing::post};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    api::{response::ApiResponse, response::ApiResult, ValidatedJson},
    auth::{self, LoginParams, RegisterParams},
    state::AppState,
};

pub fn api() -> axum::Router<Arc<AppState>> {
    axum::Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
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

#[derive(Deserialize, Validate)]
struct LoginRequest {
    #[validate(length(min = 1, max = 64))]
    username: String,
    #[validate(length(min = 8, max = 32))]
    password: String,
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
struct AuthResponse {
    token: String,
    user: UserResponse,
}

impl From<crate::db::models::User> for UserResponse {
    fn from(user: crate::db::models::User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            display_name: user.display_name,
            birthday: user.birthday,
            max_proactive_per_day: user.max_proactive_per_day as u32,
        }
    }
}

async fn register(
    State(state): State<Arc<AppState>>,
    ValidatedJson(body): ValidatedJson<RegisterRequest>,
) -> ApiResult<AuthResponse> {
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

    Ok(ApiResponse::created(AuthResponse {
        token: session.token,
        user: session.user.into(),
    }))
}

async fn login(
    State(state): State<Arc<AppState>>,
    ValidatedJson(body): ValidatedJson<LoginRequest>,
) -> ApiResult<AuthResponse> {
    let session = auth::login(
        &state.db,
        &state.config.auth,
        LoginParams {
            username: body.username,
            password: body.password,
        },
    )
    .await?;

    Ok(ApiResponse::ok(AuthResponse {
        token: session.token,
        user: session.user.into(),
    }))
}
