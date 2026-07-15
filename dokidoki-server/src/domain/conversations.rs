use chrono::{DateTime, Utc};
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::{
    db::{
        models::{Conversation, ConversationListRow},
        queries::{characters as character_queries, conversations as conversation_queries},
    },
    error::AppError,
};

pub struct LastMessagePreview {
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub role: String,
}

impl From<(String, DateTime<Utc>, String)> for LastMessagePreview {
    fn from((content, created_at, role): (String, DateTime<Utc>, String)) -> Self {
        Self {
            content,
            created_at,
            role,
        }
    }
}

pub struct ConversationListItem {
    pub id: String,
    pub character_id: String,
    pub character_name: String,
    pub status: String,
    pub last_message: Option<LastMessagePreview>,
    pub current_activity: Option<String>,
}

impl From<ConversationListRow> for ConversationListItem {
    fn from(row: ConversationListRow) -> Self {
        let last_message = match (
            row.last_message_content,
            row.last_message_created_at,
            row.last_message_role,
        ) {
            (Some(content), Some(created_at), Some(role)) => {
                Some(LastMessagePreview::from((content, created_at, role)))
            }
            _ => None,
        };

        Self {
            id: row.id,
            character_id: row.character_id,
            character_name: row.character_name,
            status: row.status.as_str().to_owned(),
            last_message,
            current_activity: row.current_activity.filter(|activity| !activity.is_empty()),
        }
    }
}

pub enum GetOrCreateConversationResult {
    Created(Conversation),
    Existing(Conversation),
}

pub async fn list_for_user(
    pool: &MySqlPool,
    user_id: &str,
) -> Result<Vec<ConversationListItem>, AppError> {
    let rows = conversation_queries::list_by_user(pool, user_id).await?;
    Ok(rows.into_iter().map(ConversationListItem::from).collect())
}

pub async fn get_or_create(
    pool: &MySqlPool,
    user_id: &str,
    character_id: &str,
) -> Result<GetOrCreateConversationResult, AppError> {
    if let Some(conversation) =
        conversation_queries::find_by_user_and_character(pool, user_id, character_id).await?
    {
        return Ok(GetOrCreateConversationResult::Existing(conversation));
    }

    character_queries::find_by_id(pool, character_id)
        .await?
        .ok_or_else(|| AppError::not_found("角色不存在"))?;

    let conversation_id = Uuid::new_v4().to_string();
    match conversation_queries::insert(pool, &conversation_id, user_id, character_id).await {
        Ok(conversation) => {
            tracing::info!(
                conversation_id = %conversation.id,
                user_id,
                character_id,
                "conversation created"
            );
            Ok(GetOrCreateConversationResult::Created(conversation))
        }
        Err(err) => {
            if let Some(conversation) =
                conversation_queries::find_by_user_and_character(pool, user_id, character_id)
                    .await?
            {
                return Ok(GetOrCreateConversationResult::Existing(conversation));
            }
            Err(err)
        }
    }
}

pub async fn require_owned(
    pool: &MySqlPool,
    conversation_id: &str,
    user_id: &str,
) -> Result<Conversation, AppError> {
    conversation_queries::find_by_id_for_user(pool, conversation_id, user_id)
        .await?
        .ok_or_else(|| AppError::not_found("会话不存在"))
}
