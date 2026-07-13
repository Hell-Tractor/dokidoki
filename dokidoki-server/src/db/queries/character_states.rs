use chrono::{DateTime, NaiveDate, Utc};
use sqlx::MySqlPool;

use crate::error::AppError;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CharacterStateReplyRow {
    pub availability: String,
    pub activity_ends_at: Option<DateTime<Utc>>,
}

pub async fn find_reply_fields(
    pool: &MySqlPool,
    character_id: &str,
) -> Result<Option<CharacterStateReplyRow>, AppError> {
    let row = sqlx::query_as::<_, CharacterStateReplyRow>(
        r#"
        SELECT availability, activity_ends_at
        FROM character_states
        WHERE character_id = ?
        "#,
    )
    .bind(character_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::from_db)?;

    Ok(row)
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CharacterStatePromptRow {
    pub current_activity: String,
    pub current_mood: String,
    pub availability: String,
    pub random_event: Option<String>,
}

pub async fn find_prompt_fields(
    pool: &MySqlPool,
    character_id: &str,
) -> Result<Option<CharacterStatePromptRow>, AppError> {
    let row = sqlx::query_as::<_, CharacterStatePromptRow>(
        r#"
        SELECT current_activity, current_mood, availability, random_event
        FROM character_states
        WHERE character_id = ?
        "#,
    )
    .bind(character_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::from_db)?;

    Ok(row)
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CharacterStateRow {
    pub random_event: Option<String>,
    pub random_event_date: Option<NaiveDate>,
}

pub async fn find_by_character_id(
    pool: &MySqlPool,
    character_id: &str,
) -> Result<Option<CharacterStateRow>, AppError> {
    let row = sqlx::query_as::<_, CharacterStateRow>(
        r#"
        SELECT random_event, random_event_date
        FROM character_states
        WHERE character_id = ?
        "#,
    )
    .bind(character_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::from_db)?;

    Ok(row)
}

pub struct UpsertStateParams<'a> {
    pub current_activity: &'a str,
    pub current_mood: &'a str,
    pub availability: &'a str,
    pub activity_ends_at: Option<DateTime<Utc>>,
    pub random_event: Option<&'a str>,
    pub random_event_date: Option<NaiveDate>,
}

pub async fn upsert(
    pool: &MySqlPool,
    character_id: &str,
    params: UpsertStateParams<'_>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO character_states (
            character_id, current_activity, current_mood, availability,
            activity_ends_at, random_event, random_event_date
        )
        VALUES (?, ?, ?, ?, ?, ?, ?)
        ON DUPLICATE KEY UPDATE
            current_activity = VALUES(current_activity),
            current_mood = VALUES(current_mood),
            availability = VALUES(availability),
            activity_ends_at = VALUES(activity_ends_at),
            random_event = VALUES(random_event),
            random_event_date = VALUES(random_event_date)
        "#,
    )
    .bind(character_id)
    .bind(params.current_activity)
    .bind(params.current_mood)
    .bind(params.availability)
    .bind(params.activity_ends_at)
    .bind(params.random_event)
    .bind(params.random_event_date)
    .execute(pool)
    .await
    .map_err(AppError::from_db)?;

    Ok(())
}
