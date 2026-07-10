use chrono::NaiveDate;
use sqlx::MySqlPool;

use crate::{
    db::{models::User, queries::users as user_queries},
    error::AppError,
};

pub struct UpdateProfileInput {
    pub display_name: Option<String>,
    pub birthday: Option<NaiveDate>,
    pub max_proactive_per_day: Option<u32>,
}

pub async fn update_profile(
    pool: &MySqlPool,
    user: &User,
    input: UpdateProfileInput,
) -> Result<User, AppError> {
    user_queries::update_profile(
        pool,
        &user.id,
        user,
        user_queries::UpdateMeParams {
            display_name: input.display_name,
            birthday: input.birthday,
            max_proactive_per_day: input.max_proactive_per_day.map(|value| value as i32),
        },
    )
    .await
}
