use std::sync::Arc;

use axum::{extract::State, routing::post};
use chrono::NaiveDate;
use serde::Deserialize;
use validator::Validate;

use crate::{
    api::{response::ApiResponse, response::ApiResult, ValidatedJson},
    domain::auth::{self, AuthSession, LoginInput, RegisterInput},
    state::AppState,
};

use super::users::UserResponse;

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

impl From<RegisterRequest> for RegisterInput {
    fn from(body: RegisterRequest) -> Self {
        Self {
            username: body.username,
            password: body.password,
            display_name: body.display_name,
            birthday: body.birthday,
        }
    }
}

impl From<LoginRequest> for LoginInput {
    fn from(body: LoginRequest) -> Self {
        Self {
            username: body.username,
            password: body.password,
        }
    }
}

#[derive(serde::Serialize)]
struct AuthResponse {
    token: String,
    user: UserResponse,
}

impl From<AuthSession> for AuthResponse {
    fn from(session: AuthSession) -> Self {
        Self {
            token: session.token,
            user: session.user.into(),
        }
    }
}

async fn register(
    State(state): State<Arc<AppState>>,
    ValidatedJson(body): ValidatedJson<RegisterRequest>,
) -> ApiResult<AuthResponse> {
    let session = auth::register(&state.db, &state.config.auth, body.into()).await?;
    Ok(ApiResponse::created(session.into()))
}

async fn login(
    State(state): State<Arc<AppState>>,
    ValidatedJson(body): ValidatedJson<LoginRequest>,
) -> ApiResult<AuthResponse> {
    let session = auth::login(&state.db, &state.config.auth, body.into()).await?;
    Ok(ApiResponse::ok(session.into()))
}
