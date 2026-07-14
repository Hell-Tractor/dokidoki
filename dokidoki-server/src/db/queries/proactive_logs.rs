use chrono::{DateTime, Utc};
use sqlx::MySqlPool;

use crate::error::AppError;

pub async fn count_for_user_between(
    pool: &MySqlPool,
    user_id: &str,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<i64, AppError> {
    let count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM proactive_logs
        WHERE user_id = ?
          AND created_at >= ?
          AND created_at < ?
        "#,
    )
    .bind(user_id)
    .bind(start)
    .bind(end)
    .fetch_one(pool)
    .await
    .map_err(AppError::from_db)?;

    Ok(count)
}

/// 统计某用户-角色在区间内某触发类型是否已发送（用于每日问候等）。
pub async fn count_trigger_between(
    pool: &MySqlPool,
    user_id: &str,
    character_id: &str,
    trigger_type: &str,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<i64, AppError> {
    let count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM proactive_logs
        WHERE user_id = ?
          AND character_id = ?
          AND trigger_type = ?
          AND created_at >= ?
          AND created_at < ?
        "#,
    )
    .bind(user_id)
    .bind(character_id)
    .bind(trigger_type)
    .bind(start)
    .bind(end)
    .fetch_one(pool)
    .await
    .map_err(AppError::from_db)?;

    Ok(count)
}

pub async fn insert(
    pool: &MySqlPool,
    id: &str,
    user_id: &str,
    character_id: &str,
    conversation_id: &str,
    trigger_type: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO proactive_logs (id, user_id, character_id, conversation_id, trigger_type)
        VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(id)
    .bind(user_id)
    .bind(character_id)
    .bind(conversation_id)
    .bind(trigger_type)
    .execute(pool)
    .await
    .map_err(AppError::from_db)?;

    Ok(())
}
