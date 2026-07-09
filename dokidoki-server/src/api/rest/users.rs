use std::sync::Arc;

use axum::{
    routing::get,
    Router,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    api::{
        extractors::{AuthUser, ValidatedJson},
        response::{ApiResponse, ApiResult},
    },
    db::queries::users::{self, UpdateMeParams},
    state::AppState,
};

pub fn api() -> Router<Arc<AppState>> {
    Router::new()
        .route("/me", get(get_me).patch(patch_me))
}

#[derive(Serialize)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub birthday: Option<NaiveDate>,
    pub max_proactive_per_day: u32,
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

#[derive(Deserialize, Validate, Default)]
struct UpdateMeRequest {
    #[serde(default)]
    #[validate(length(min = 1, max = 64))]
    display_name: Option<String>,
    #[serde(default)]
    birthday: Option<NaiveDate>,
    #[serde(default)]
    #[validate(range(min = 0, max = 100))]
    max_proactive_per_day: Option<u32>,
}

async fn get_me(AuthUser(user): AuthUser) -> ApiResult<UserResponse> {
    Ok(ApiResponse::ok(user.into()))
}

async fn patch_me(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    ValidatedJson(body): ValidatedJson<UpdateMeRequest>,
) -> ApiResult<UserResponse> {
    let user = users::update_profile(
        &state.db,
        &user.id,
        &user,
        UpdateMeParams {
            display_name: body.display_name,
            birthday: body.birthday,
            max_proactive_per_day: body.max_proactive_per_day.map(|value| value as i32),
        },
    )
    .await?;

    Ok(ApiResponse::ok(user.into()))
}
