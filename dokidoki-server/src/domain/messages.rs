use chrono::{DateTime, Utc};
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::{
    db::{message::Message, queries::messages as message_queries},
    domain::conversations,
    error::AppError,
};

pub struct SentTextMessage {
    pub id: String,
    pub turn_id: String,
    pub created_at: DateTime<Utc>,
}

pub struct MessagePage {
    pub messages: Vec<Message>,
    pub has_more: bool,
}

fn normalize_text_content(content: &str) -> Result<&str, AppError> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err(AppError::bad_request("消息内容不能为空"));
    }
    Ok(trimmed)
}

pub async fn list_for_conversation(
    pool: &MySqlPool,
    user_id: &str,
    conversation_id: &str,
    before: Option<&str>,
    limit: u32,
) -> Result<MessagePage, AppError> {
    conversations::require_owned(pool, conversation_id, user_id).await?;

    let (messages, has_more) =
        message_queries::list_page(pool, conversation_id, before, limit).await?;

    Ok(MessagePage {
        messages,
        has_more,
    })
}

pub async fn send_user_text(
    pool: &MySqlPool,
    user_id: &str,
    conversation_id: &str,
    content: String,
) -> Result<SentTextMessage, AppError> {
    conversations::require_owned(pool, conversation_id, user_id).await?;

    let content = normalize_text_content(&content)?;
    let message_id = Uuid::new_v4().to_string();
    let turn_id = Uuid::new_v4().to_string();
    let message = message_queries::insert_user_text(
        pool,
        &message_id,
        conversation_id,
        content,
        &turn_id,
    )
    .await?;

    Ok(SentTextMessage {
        id: message.id,
        turn_id: message.turn_id.unwrap_or(turn_id),
        created_at: message.created_at,
    })
}
