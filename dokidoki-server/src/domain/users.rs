use chrono::NaiveDate;
use sqlx::MySqlPool;

use crate::{
    db::{models::User, queries::users as user_queries},
    error::AppError,
};

pub struct UpdateProfileInput {
    pub display_name: Option<String>,
    pub birthday: Option<NaiveDate>,
    pub timezone: Option<String>,
    pub max_proactive_per_day: Option<u32>,
}

pub async fn update_profile(
    pool: &MySqlPool,
    user: &User,
    input: UpdateProfileInput,
) -> Result<User, AppError> {
    let timezone = match input.timezone {
        Some(ref tz) => Some(crate::time::parse_timezone(tz)?.to_string()),
        None => None,
    };

    let updated = user_queries::update_profile(
        pool,
        &user.id,
        user,
        user_queries::UpdateMeParams {
            display_name: input.display_name.clone(),
            birthday: input.birthday,
            timezone: timezone.clone(),
            max_proactive_per_day: input.max_proactive_per_day.map(|value| value as i32),
        },
    )
    .await?;

    tracing::info!(
        user_id = %user.id,
        display_name_set = input.display_name.is_some(),
        birthday_set = input.birthday.is_some(),
        timezone = ?timezone,
        max_proactive_per_day = ?input.max_proactive_per_day,
        "user profile updated"
    );
    Ok(updated)
}
