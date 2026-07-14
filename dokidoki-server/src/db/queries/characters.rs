use sqlx::MySqlPool;

use crate::{
    db::models::Character,
    error::AppError,
};

pub async fn list_all(pool: &MySqlPool) -> Result<Vec<Character>, AppError> {
    let characters = sqlx::query_as::<_, Character>(
        r#"
        SELECT id, name, avatar_path
        FROM characters
        ORDER BY name ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::from_db)?;

    Ok(characters)
}

pub async fn list_character_ids(pool: &MySqlPool) -> Result<Vec<String>, AppError> {
    let ids = sqlx::query_scalar::<_, String>(
        r#"
        SELECT id
        FROM characters
        ORDER BY id ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::from_db)?;

    Ok(ids)
}

pub async fn find_persona(
    pool: &MySqlPool,
    id: &str,
) -> Result<Option<crate::domain::persona::Persona>, AppError> {
    let raw = sqlx::query_scalar::<_, Option<serde_json::Value>>(
        r#"
        SELECT persona_json
        FROM characters
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::from_db)?
    .flatten();

    match raw {
        None => Ok(None),
        Some(value) => crate::domain::persona::Persona::from_json_value(value)
            .map(Some)
            .map_err(|err| {
                tracing::warn!(character_id = %id, "invalid persona_json: {err}");
                AppError::internal(err)
            }),
    }
}

pub async fn find_schedule_json(
    pool: &MySqlPool,
    id: &str,
) -> Result<Option<serde_json::Value>, AppError> {
    let schedule = sqlx::query_scalar::<_, Option<serde_json::Value>>(
        r#"
        SELECT schedule_json
        FROM characters
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::from_db)?
    .flatten();

    Ok(schedule)
}

pub async fn find_by_id(pool: &MySqlPool, id: &str) -> Result<Option<Character>, AppError> {
    let character = sqlx::query_as::<_, Character>(
        r#"
        SELECT id, name, avatar_path
        FROM characters
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::from_db)?;

    Ok(character)
}
