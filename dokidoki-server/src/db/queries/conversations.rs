use sqlx::MySqlPool;

use crate::{
    db::models::{Conversation, ConversationListRow},
    error::AppError,
};

pub async fn list_by_user(
    pool: &MySqlPool,
    user_id: &str,
) -> Result<Vec<ConversationListRow>, AppError> {
    let rows = sqlx::query_as::<_, ConversationListRow>(
        r#"
        SELECT
            c.id,
            c.character_id,
            ch.name AS character_name,
            c.status,
            cs.current_activity,
            lm.content AS last_message_content,
            lm.created_at AS last_message_created_at,
            lm.role AS last_message_role
        FROM conversations c
        INNER JOIN characters ch ON ch.id = c.character_id
        LEFT JOIN character_states cs ON cs.character_id = c.character_id
        LEFT JOIN messages lm ON lm.id = (
            SELECT m.id
            FROM messages m
            WHERE m.conversation_id = c.id
            ORDER BY m.created_at DESC, m.id DESC
            LIMIT 1
        )
        WHERE c.user_id = ?
        ORDER BY COALESCE(lm.created_at, c.updated_at) DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::from_db)?;

    Ok(rows)
}

pub async fn find_by_user_and_character(
    pool: &MySqlPool,
    user_id: &str,
    character_id: &str,
) -> Result<Option<Conversation>, AppError> {
    let conversation = sqlx::query_as::<_, Conversation>(
        r#"
        SELECT id, user_id, character_id, status, first_contact_done
        FROM conversations
        WHERE user_id = ? AND character_id = ?
        "#,
    )
    .bind(user_id)
    .bind(character_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::from_db)?;

    Ok(conversation)
}

pub async fn find_by_id_for_user(
    pool: &MySqlPool,
    conversation_id: &str,
    user_id: &str,
) -> Result<Option<Conversation>, AppError> {
    let conversation = sqlx::query_as::<_, Conversation>(
        r#"
        SELECT id, user_id, character_id, status, first_contact_done
        FROM conversations
        WHERE id = ? AND user_id = ?
        "#,
    )
    .bind(conversation_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::from_db)?;

    Ok(conversation)
}

pub async fn find_by_id(pool: &MySqlPool, id: &str) -> Result<Option<Conversation>, AppError> {
    sqlx::query_as::<_, Conversation>(
        r#"
        SELECT id, user_id, character_id, status, first_contact_done
        FROM conversations
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::from_db)
}

pub async fn insert(
    pool: &MySqlPool,
    id: &str,
    user_id: &str,
    character_id: &str,
) -> Result<Conversation, AppError> {
    sqlx::query(
        r#"
        INSERT INTO conversations (id, user_id, character_id)
        VALUES (?, ?, ?)
        "#,
    )
    .bind(id)
    .bind(user_id)
    .bind(character_id)
    .execute(pool)
    .await
    .map_err(AppError::from_db)?;

    fetch_by_id(pool, id).await
}

async fn fetch_by_id(pool: &MySqlPool, id: &str) -> Result<Conversation, AppError> {
    find_by_id(pool, id)
        .await?
        .ok_or_else(|| AppError::internal(std::io::Error::other("conversation not found after insert")))
}

pub async fn try_begin_icebreaker(pool: &MySqlPool, conversation_id: &str) -> Result<bool, AppError> {
    let result = sqlx::query(
        r#"
        UPDATE conversations
        SET first_contact_done = 1
        WHERE id = ? AND first_contact_done = 0
        "#,
    )
    .bind(conversation_id)
    .execute(pool)
    .await
    .map_err(AppError::from_db)?;

    Ok(result.rows_affected() > 0)
}

pub async fn rollback_icebreaker(pool: &MySqlPool, conversation_id: &str) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE conversations
        SET first_contact_done = 0
        WHERE id = ?
        "#,
    )
    .bind(conversation_id)
    .execute(pool)
    .await
    .map_err(AppError::from_db)?;

    Ok(())
}

pub async fn update_status(
    pool: &MySqlPool,
    conversation_id: &str,
    status: &str,
    set_paused_at: bool,
) -> Result<(), AppError> {
    if set_paused_at {
        sqlx::query(
            r#"
            UPDATE conversations
            SET status = ?, paused_at = UTC_TIMESTAMP(6)
            WHERE id = ?
            "#,
        )
        .bind(status)
        .bind(conversation_id)
        .execute(pool)
        .await
        .map_err(AppError::from_db)?;
    } else {
        sqlx::query(
            r#"
            UPDATE conversations
            SET status = ?, paused_at = NULL
            WHERE id = ?
            "#,
        )
        .bind(status)
        .bind(conversation_id)
        .execute(pool)
        .await
        .map_err(AppError::from_db)?;
    }

    Ok(())
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ConversationSummaryFields {
    pub summary: Option<String>,
    pub summary_covers_until: Option<chrono::DateTime<chrono::Utc>>,
}

pub async fn find_summary_fields(
    pool: &MySqlPool,
    conversation_id: &str,
) -> Result<Option<ConversationSummaryFields>, AppError> {
    let row = sqlx::query_as::<_, ConversationSummaryFields>(
        r#"
        SELECT summary, summary_covers_until
        FROM conversations
        WHERE id = ?
        "#,
    )
    .bind(conversation_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::from_db)?;

    Ok(row)
}

pub async fn update_summary(
    pool: &MySqlPool,
    conversation_id: &str,
    summary: &str,
    covers_until: chrono::DateTime<chrono::Utc>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE conversations
        SET summary = ?, summary_covers_until = ?
        WHERE id = ?
        "#,
    )
    .bind(summary)
    .bind(covers_until)
    .bind(conversation_id)
    .execute(pool)
    .await
    .map_err(AppError::from_db)?;

    Ok(())
}
