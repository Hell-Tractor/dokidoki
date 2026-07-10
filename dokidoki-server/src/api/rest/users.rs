use std::sync::Arc;

use axum::{routing::get, Router};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    api::{
        extractors::{AuthUser, ValidatedJson},
        response::{ApiResponse, ApiResult},
    },
    db::models::User,
    domain::users::{self, UpdateProfileInput},
    state::AppState,
    time::is_valid_timezone,
};

pub fn api() -> Router<Arc<AppState>> {
    Router::new()
        .route("/me", get(get_me).patch(patch_me))
}

#[derive(Serialize)]
pub struct UserResponse {
    id: String,
    username: String,
    display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    birthday: Option<NaiveDate>,
    timezone: String,
    max_proactive_per_day: u32,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            display_name: user.display_name,
            birthday: user.birthday,
            timezone: user.timezone,
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
    #[validate(custom(function = "validate_timezone_field"))]
    timezone: Option<String>,
    #[serde(default)]
    #[validate(range(min = 0, max = 100))]
    max_proactive_per_day: Option<u32>,
}

fn validate_timezone_field(timezone: &String) -> Result<(), validator::ValidationError> {
    if is_valid_timezone(timezone) {
        Ok(())
    } else {
        Err(validator::ValidationError::new("invalid_timezone"))
    }
}

impl From<UpdateMeRequest> for UpdateProfileInput {
    fn from(body: UpdateMeRequest) -> Self {
        Self {
            display_name: body.display_name,
            birthday: body.birthday,
            timezone: body.timezone,
            max_proactive_per_day: body.max_proactive_per_day,
        }
    }
}

async fn get_me(AuthUser(user): AuthUser) -> ApiResult<UserResponse> {
    Ok(ApiResponse::ok(user.into()))
}

async fn patch_me(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    ValidatedJson(body): ValidatedJson<UpdateMeRequest>,
) -> ApiResult<UserResponse> {
    let user = users::update_profile(&state.db, &user, body.into()).await?;
    Ok(ApiResponse::ok(user.into()))
}
