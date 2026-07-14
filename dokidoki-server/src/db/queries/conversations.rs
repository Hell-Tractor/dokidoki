use chrono::NaiveTime;
use sqlx::MySqlPool;

use crate::{
    db::models::{Conversation, ConversationListRow},
    error::AppError,
};

/// 主动消息 tick 候选：已破冰的会话及闸门所需字段。
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ProactiveCandidateRow {
    pub id: String,
    pub user_id: String,
    pub character_id: String,
    pub status: crate::domain::ConversationStatus,
    pub paused_at: Option<chrono::DateTime<chrono::Utc>>,
    pub timezone: String,
    pub max_proactive_per_day: i32,
    pub availability: Option<crate::domain::Availability>,
    pub activity_ends_at: Option<chrono::DateTime<chrono::Utc>>,
    pub dnd_start: Option<NaiveTime>,
    pub dnd_end: Option<NaiveTime>,
    /// 会话内最近一条用户消息时间；无用户消息则为 `None`。
    pub last_user_message_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub async fn list_proactive_candidates(
    pool: &MySqlPool,
) -> Result<Vec<ProactiveCandidateRow>, AppError> {
    let rows = sqlx::query_as::<_, ProactiveCandidateRow>(
        r#"
        SELECT
            c.id,
            c.user_id,
            c.character_id,
            c.status,
            c.paused_at,
            u.timezone,
            u.max_proactive_per_day,
            cs.availability,
            cs.activity_ends_at,
            ucs.dnd_start,
            ucs.dnd_end,
            (
                SELECT m.created_at
                FROM messages m
                WHERE m.conversation_id = c.id
                  AND m.role = 'user'
                ORDER BY m.created_at DESC, m.id DESC
                LIMIT 1
            ) AS last_user_message_at
        FROM conversations c
        INNER JOIN users u ON u.id = c.user_id
        LEFT JOIN character_states cs ON cs.character_id = c.character_id
        LEFT JOIN user_character_settings ucs
            ON ucs.user_id = c.user_id AND ucs.character_id = c.character_id
        WHERE c.first_contact_done = 1
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::from_db)?;

    Ok(rows)
}

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
        SELECT id, user_id, character_id, status, winding_reason, first_contact_done
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
        SELECT id, user_id, character_id, status, winding_reason, first_contact_done
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
        SELECT id, user_id, character_id, status, winding_reason, first_contact_done
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

#[derive(Debug, Clone, Copy)]
enum StatusField<T> {
    Clear,
    Set(T),
    SetNow,
}

#[derive(Debug, Clone, Copy)]
struct StatusUpdate {
    status: crate::domain::ConversationStatus,
    winding_reason: StatusField<crate::domain::WindingReason>,
    winding_started_at: StatusField<()>,
    paused_at: StatusField<()>,
}

async fn apply_status_update(
    pool: &MySqlPool,
    conversation_id: &str,
    update: StatusUpdate,
) -> Result<(), AppError> {
    let clear_winding_started = matches!(update.winding_started_at, StatusField::Clear);
    let set_winding_started = matches!(update.winding_started_at, StatusField::SetNow);
    let clear_paused = matches!(update.paused_at, StatusField::Clear);
    let set_paused = matches!(update.paused_at, StatusField::SetNow);

    sqlx::query(
        r#"
        UPDATE conversations
        SET
            status = ?,
            winding_reason = CASE
                WHEN ? = 1 THEN NULL
                WHEN ? = 1 THEN ?
                ELSE winding_reason
            END,
            winding_started_at = CASE
                WHEN ? = 1 THEN NULL
                WHEN ? = 1 THEN UTC_TIMESTAMP(6)
                ELSE winding_started_at
            END,
            paused_at = CASE
                WHEN ? = 1 THEN NULL
                WHEN ? = 1 THEN UTC_TIMESTAMP(6)
                ELSE paused_at
            END
        WHERE id = ?
        "#,
    )
    .bind(update.status)
    .bind(matches!(update.winding_reason, StatusField::Clear) as i32)
    .bind(matches!(update.winding_reason, StatusField::Set(_)) as i32)
    .bind(match update.winding_reason {
        StatusField::Set(v) => Some(v),
        _ => None,
    })
    .bind(clear_winding_started as i32)
    .bind(set_winding_started as i32)
    .bind(clear_paused as i32)
    .bind(set_paused as i32)
    .bind(conversation_id)
    .execute(pool)
    .await
    .map_err(AppError::from_db)?;

    Ok(())
}

/// 进入 winding_down 并记录原因与起点。
pub async fn enter_winding_down(
    pool: &MySqlPool,
    conversation_id: &str,
    reason: crate::domain::WindingReason,
) -> Result<(), AppError> {
    apply_status_update(
        pool,
        conversation_id,
        StatusUpdate {
            status: crate::domain::ConversationStatus::WindingDown,
            winding_reason: StatusField::Set(reason),
            winding_started_at: StatusField::SetNow,
            paused_at: StatusField::Clear,
        },
    )
    .await
}

/// 进入终态暂停（paused / paused_char_busy / paused_user_busy）。
pub async fn enter_terminal_pause(
    pool: &MySqlPool,
    conversation_id: &str,
    status: crate::domain::ConversationStatus,
) -> Result<(), AppError> {
    apply_status_update(
        pool,
        conversation_id,
        StatusUpdate {
            status,
            winding_reason: StatusField::Clear,
            winding_started_at: StatusField::Clear,
            paused_at: StatusField::SetNow,
        },
    )
    .await
}

/// 恢复 active，清空 winding / paused 元数据。
pub async fn enter_active(pool: &MySqlPool, conversation_id: &str) -> Result<(), AppError> {
    apply_status_update(
        pool,
        conversation_id,
        StatusUpdate {
            status: crate::domain::ConversationStatus::Active,
            winding_reason: StatusField::Clear,
            winding_started_at: StatusField::Clear,
            paused_at: StatusField::Clear,
        },
    )
    .await
}

/// 列出已超时的 winding_down 会话。
pub async fn list_winding_down_timed_out(
    pool: &MySqlPool,
    older_than: chrono::DateTime<chrono::Utc>,
) -> Result<Vec<(String, Option<crate::domain::WindingReason>)>, AppError> {
    #[derive(sqlx::FromRow)]
    struct Row {
        id: String,
        winding_reason: Option<crate::domain::WindingReason>,
    }

    let rows = sqlx::query_as::<_, Row>(
        r#"
        SELECT id, winding_reason
        FROM conversations
        WHERE status = 'winding_down'
          AND winding_started_at IS NOT NULL
          AND winding_started_at <= ?
        "#,
    )
    .bind(older_than)
    .fetch_all(pool)
    .await
    .map_err(AppError::from_db)?;

    Ok(rows
        .into_iter()
        .map(|row| (row.id, row.winding_reason))
        .collect())
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
