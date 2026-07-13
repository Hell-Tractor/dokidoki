use sqlx::MySqlPool;

use crate::{
    db::message::Message,
    error::AppError,
};

pub async fn find_by_id_in_conversation(
    pool: &MySqlPool,
    conversation_id: &str,
    message_id: &str,
) -> Result<Option<Message>, AppError> {
    let message = sqlx::query_as::<_, Message>(
        r#"
        SELECT
            id,
            role,
            content,
            content_type,
            turn_id,
            seq_in_turn,
            metadata,
            reply_to_id,
            read_at,
            created_at
        FROM messages
        WHERE conversation_id = ? AND id = ?
        "#,
    )
    .bind(conversation_id)
    .bind(message_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::from_db)?;

    Ok(message)
}

pub async fn list_page(
    pool: &MySqlPool,
    conversation_id: &str,
    before: Option<&str>,
    limit: u32,
) -> Result<(Vec<Message>, bool), AppError> {
    let fetch_limit = i64::from(limit) + 1;

    let mut rows = if let Some(before_id) = before {
        let cursor = find_by_id_in_conversation(pool, conversation_id, before_id)
            .await?
            .ok_or_else(|| AppError::bad_request("before 游标无效"))?;

        sqlx::query_as::<_, Message>(
            r#"
            SELECT
                id,
                role,
                content,
                content_type,
                turn_id,
                seq_in_turn,
                metadata,
                reply_to_id,
                read_at,
                created_at
            FROM messages
            WHERE conversation_id = ?
              AND (created_at < ? OR (created_at = ? AND id < ?))
            ORDER BY created_at DESC, id DESC
            LIMIT ?
            "#,
        )
        .bind(conversation_id)
        .bind(cursor.created_at)
        .bind(cursor.created_at)
        .bind(before_id)
        .bind(fetch_limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::from_db)?
    } else {
        sqlx::query_as::<_, Message>(
            r#"
            SELECT
                id,
                role,
                content,
                content_type,
                turn_id,
                seq_in_turn,
                metadata,
                reply_to_id,
                read_at,
                created_at
            FROM messages
            WHERE conversation_id = ?
            ORDER BY created_at DESC, id DESC
            LIMIT ?
            "#,
        )
        .bind(conversation_id)
        .bind(fetch_limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::from_db)?
    };

    let has_more = rows.len() > limit as usize;
    if has_more {
        rows.pop();
    }
    rows.reverse();
    Ok((rows, has_more))
}

pub async fn list_recent_text(
    pool: &MySqlPool,
    conversation_id: &str,
    limit: u32,
) -> Result<Vec<Message>, AppError> {
    let mut rows = sqlx::query_as::<_, Message>(
        r#"
        SELECT
            id,
            role,
            content,
            content_type,
            turn_id,
            seq_in_turn,
            metadata,
            reply_to_id,
            read_at,
            created_at
        FROM messages
        WHERE conversation_id = ?
          AND content_type = 'text'
        ORDER BY created_at DESC, id DESC
        LIMIT ?
        "#,
    )
    .bind(conversation_id)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(AppError::from_db)?;

    rows.reverse();
    Ok(rows)
}

pub async fn insert_character_text(
    pool: &MySqlPool,
    id: &str,
    conversation_id: &str,
    content: &str,
    turn_id: &str,
    seq_in_turn: i32,
    reply_to_id: Option<&str>,
) -> Result<Message, AppError> {
    sqlx::query(
        r#"
        INSERT INTO messages (
            id, conversation_id, role, content, content_type, turn_id, seq_in_turn, reply_to_id
        )
        VALUES (?, ?, 'character', ?, 'text', ?, ?, ?)
        "#,
    )
    .bind(id)
    .bind(conversation_id)
    .bind(content)
    .bind(turn_id)
    .bind(seq_in_turn)
    .bind(reply_to_id)
    .execute(pool)
    .await
    .map_err(AppError::from_db)?;

    let message = sqlx::query_as::<_, Message>(
        r#"
        SELECT
            id,
            role,
            content,
            content_type,
            turn_id,
            seq_in_turn,
            metadata,
            reply_to_id,
            read_at,
            created_at
        FROM messages
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(AppError::from_db)?;

    Ok(message)
}

pub async fn insert_user_text(
    pool: &MySqlPool,
    id: &str,
    conversation_id: &str,
    content: &str,
    turn_id: &str,
) -> Result<Message, AppError> {
    sqlx::query(
        r#"
        INSERT INTO messages (id, conversation_id, role, content, content_type, turn_id)
        VALUES (?, ?, 'user', ?, 'text', ?)
        "#,
    )
    .bind(id)
    .bind(conversation_id)
    .bind(content)
    .bind(turn_id)
    .execute(pool)
    .await
    .map_err(AppError::from_db)?;

    let message = sqlx::query_as::<_, Message>(
        r#"
        SELECT
            id,
            role,
            content,
            content_type,
            turn_id,
            seq_in_turn,
            metadata,
            reply_to_id,
            read_at,
            created_at
        FROM messages
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(AppError::from_db)?;

    Ok(message)
}

pub async fn insert_user_burst_text(
    pool: &MySqlPool,
    id: &str,
    conversation_id: &str,
    content: &str,
    turn_id: &str,
    seq_in_turn: i32,
) -> Result<Message, AppError> {
    sqlx::query(
        r#"
        INSERT INTO messages (
            id, conversation_id, role, content, content_type, turn_id, seq_in_turn, is_burst_part
        )
        VALUES (?, ?, 'user', ?, 'text', ?, ?, 1)
        "#,
    )
    .bind(id)
    .bind(conversation_id)
    .bind(content)
    .bind(turn_id)
    .bind(seq_in_turn)
    .execute(pool)
    .await
    .map_err(AppError::from_db)?;

    let message = sqlx::query_as::<_, Message>(
        r#"
        SELECT
            id,
            role,
            content,
            content_type,
            turn_id,
            seq_in_turn,
            metadata,
            reply_to_id,
            read_at,
            created_at
        FROM messages
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(AppError::from_db)?;

    Ok(message)
}

pub async fn mark_user_messages_read(
    pool: &MySqlPool,
    conversation_id: &str,
    user_id: &str,
    message_ids: &[String],
) -> Result<Option<chrono::DateTime<chrono::Utc>>, AppError> {
    if message_ids.is_empty() {
        return Ok(None);
    }

    let read_at = chrono::Utc::now();
    let mut affected = 0u64;

    for message_id in message_ids {
        let result = sqlx::query(
            r#"
            UPDATE messages m
            INNER JOIN conversations c ON c.id = m.conversation_id
            SET m.read_at = ?
            WHERE m.conversation_id = ?
              AND c.user_id = ?
              AND m.role = 'user'
              AND m.read_at IS NULL
              AND m.id = ?
            "#,
        )
        .bind(read_at)
        .bind(conversation_id)
        .bind(user_id)
        .bind(message_id)
        .execute(pool)
        .await
        .map_err(AppError::from_db)?;

        affected += result.rows_affected();
    }

    if affected == 0 {
        return Ok(None);
    }

    Ok(Some(read_at))
}
