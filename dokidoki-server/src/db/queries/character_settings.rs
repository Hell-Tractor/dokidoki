use chrono::NaiveTime;
use sqlx::MySqlPool;

use crate::{
    db::models::UserCharacterSettings,
    error::AppError,
};

pub async fn find_by_user_and_character(
    pool: &MySqlPool,
    user_id: &str,
    character_id: &str,
) -> Result<Option<UserCharacterSettings>, AppError> {
    let settings = sqlx::query_as::<_, UserCharacterSettings>(
        r#"
        SELECT dnd_start, dnd_end, push_muted
        FROM user_character_settings
        WHERE user_id = ? AND character_id = ?
        "#,
    )
    .bind(user_id)
    .bind(character_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::from_db)?;

    Ok(settings)
}

pub struct UpsertSettingsParams {
    pub dnd_start: Option<NaiveTime>,
    pub dnd_end: Option<NaiveTime>,
}

pub async fn upsert(
    pool: &MySqlPool,
    user_id: &str,
    character_id: &str,
    params: UpsertSettingsParams,
) -> Result<UserCharacterSettings, AppError> {
    sqlx::query(
        r#"
        INSERT INTO user_character_settings (user_id, character_id, dnd_start, dnd_end)
        VALUES (?, ?, ?, ?)
        ON DUPLICATE KEY UPDATE
            dnd_start = VALUES(dnd_start),
            dnd_end = VALUES(dnd_end)
        "#,
    )
    .bind(user_id)
    .bind(character_id)
    .bind(params.dnd_start)
    .bind(params.dnd_end)
    .execute(pool)
    .await
    .map_err(AppError::from_db)?;

    find_by_user_and_character(pool, user_id, character_id)
        .await?
        .ok_or_else(|| AppError::internal(std::io::Error::other("settings not found after upsert")))
}
