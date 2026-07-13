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

pub async fn find_persona_json(
    pool: &MySqlPool,
    id: &str,
) -> Result<Option<serde_json::Value>, AppError> {
    let persona = sqlx::query_scalar::<_, Option<serde_json::Value>>(
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

    Ok(persona)
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
