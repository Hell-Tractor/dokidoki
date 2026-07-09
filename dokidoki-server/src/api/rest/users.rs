use std::sync::Arc;

use axum::{routing::get, Router};
use chrono::NaiveDate;
use serde::Serialize;

use crate::{
    api::{extractors::AuthUser, response::ApiResponse, response::ApiResult},
    state::AppState,
};

pub fn api() -> Router<Arc<AppState>> {
    Router::new().route("/me", get(get_me))
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

async fn get_me(AuthUser(user): AuthUser) -> ApiResult<UserResponse> {
    Ok(ApiResponse::ok(user.into()))
}
