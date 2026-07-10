use sqlx::{Executor, MySql, MySqlPool};

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

pub async fn insert<'e, E>(
    executor: E,
    id: &str,
    user_id: &str,
    character_id: &str,
) -> Result<Conversation, AppError>
where
    E: Executor<'e, Database = MySql>,
{
    sqlx::query(
        r#"
        INSERT INTO conversations (id, user_id, character_id)
        VALUES (?, ?, ?)
        "#,
    )
    .bind(id)
    .bind(user_id)
    .bind(character_id)
    .execute(executor)
    .await
    .map_err(AppError::from_db)?;

    Ok(Conversation {
        id: id.to_owned(),
        user_id: user_id.to_owned(),
        character_id: character_id.to_owned(),
        status: "active".to_owned(),
        first_contact_done: false,
    })
}
