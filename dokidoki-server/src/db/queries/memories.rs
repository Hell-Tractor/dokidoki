use chrono::{DateTime, Utc};
use sqlx::MySqlPool;

use crate::error::AppError;

#[derive(Debug, Clone, sqlx::FromRow)]
#[allow(unused)]
pub struct UserMemory {
    pub id: String,
    pub content: String,
    pub memory_type: String,
    pub memory_key: Option<String>,
}

pub async fn list_active(
    pool: &MySqlPool,
    user_id: &str,
    character_id: &str,
) -> Result<Vec<UserMemory>, AppError> {
    let rows = sqlx::query_as::<_, UserMemory>(
        r#"
        SELECT id, content, memory_type, memory_key
        FROM user_memories
        WHERE user_id = ?
          AND character_id = ?
          AND (expires_at IS NULL OR expires_at > UTC_TIMESTAMP(6))
        ORDER BY updated_at DESC, created_at DESC
        "#,
    )
    .bind(user_id)
    .bind(character_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::from_db)?;

    Ok(rows)
}

pub async fn upsert(
    pool: &MySqlPool,
    id: &str,
    user_id: &str,
    character_id: &str,
    content: &str,
    memory_type: &str,
    memory_key: Option<&str>,
    expires_at: Option<DateTime<Utc>>,
) -> Result<(), AppError> {
    if let Some(key) = memory_key {
        sqlx::query(
            r#"
            INSERT INTO user_memories (
                id, user_id, character_id, content, memory_type, memory_key, expires_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE
                content = VALUES(content),
                memory_type = VALUES(memory_type),
                expires_at = VALUES(expires_at),
                updated_at = CURRENT_TIMESTAMP(6)
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(character_id)
        .bind(content)
        .bind(memory_type)
        .bind(key)
        .bind(expires_at)
        .execute(pool)
        .await
        .map_err(AppError::from_db)?;
    } else {
        sqlx::query(
            r#"
            INSERT INTO user_memories (
                id, user_id, character_id, content, memory_type, memory_key, expires_at
            )
            VALUES (?, ?, ?, ?, ?, NULL, ?)
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(character_id)
        .bind(content)
        .bind(memory_type)
        .bind(expires_at)
        .execute(pool)
        .await
        .map_err(AppError::from_db)?;
    }

    Ok(())
}

pub async fn forget_by_key(
    pool: &MySqlPool,
    user_id: &str,
    character_id: &str,
    memory_key: &str,
) -> Result<u64, AppError> {
    let result = sqlx::query(
        r#"
        DELETE FROM user_memories
        WHERE user_id = ?
          AND character_id = ?
          AND memory_key = ?
        "#,
    )
    .bind(user_id)
    .bind(character_id)
    .bind(memory_key)
    .execute(pool)
    .await
    .map_err(AppError::from_db)?;

    Ok(result.rows_affected())
}

pub async fn forget_by_keyword(
    pool: &MySqlPool,
    user_id: &str,
    character_id: &str,
    keyword: &str,
) -> Result<u64, AppError> {
    let pattern = format!("%{keyword}%");
    let result = sqlx::query(
        r#"
        DELETE FROM user_memories
        WHERE user_id = ?
          AND character_id = ?
          AND content LIKE ?
        "#,
    )
    .bind(user_id)
    .bind(character_id)
    .bind(pattern)
    .execute(pool)
    .await
    .map_err(AppError::from_db)?;

    Ok(result.rows_affected())
}

pub async fn delete_expired(pool: &MySqlPool) -> Result<u64, AppError> {
    let result = sqlx::query(
        r#"
        DELETE FROM user_memories
        WHERE expires_at IS NOT NULL
          AND expires_at <= UTC_TIMESTAMP(6)
        "#,
    )
    .execute(pool)
    .await
    .map_err(AppError::from_db)?;

    Ok(result.rows_affected())
}
