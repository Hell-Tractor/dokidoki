use std::sync::Arc;

use axum::{routing::get, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::{
    api::{
        extractors::{AuthUser, ValidatedJson},
        response::{ApiResponse, ApiResult},
    },
    db::{
        models::{Conversation, ConversationListRow},
        queries::{characters, conversations},
    },
    error::AppError,
    state::AppState,
};

pub fn api() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_conversations).post(create_conversation))
        .nest("/{conversation_id}/messages", super::messages::api())
}

#[derive(Serialize)]
pub struct ConversationResponse {
    pub id: String,
    pub character_id: String,
    pub status: String,
    pub first_contact_done: bool,
}

impl From<Conversation> for ConversationResponse {
    fn from(conversation: Conversation) -> Self {
        Self {
            id: conversation.id,
            character_id: conversation.character_id,
            status: conversation.status,
            first_contact_done: conversation.first_contact_done,
        }
    }
}

#[derive(Serialize)]
pub struct LastMessageResponse {
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub role: String,
}

#[derive(Serialize)]
pub struct ConversationListItemResponse {
    pub id: String,
    pub character_id: String,
    pub character_name: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_message: Option<LastMessageResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_activity: Option<String>,
}

impl From<ConversationListRow> for ConversationListItemResponse {
    fn from(row: ConversationListRow) -> Self {
        let last_message = match (
            row.last_message_content,
            row.last_message_created_at,
            row.last_message_role,
        ) {
            (Some(content), Some(created_at), Some(role)) => Some(LastMessageResponse {
                content,
                created_at,
                role,
            }),
            _ => None,
        };

        Self {
            id: row.id,
            character_id: row.character_id,
            character_name: row.character_name,
            status: row.status,
            last_message,
            current_activity: row.current_activity.filter(|activity| !activity.is_empty()),
        }
    }
}

#[derive(Deserialize, Validate)]
struct CreateConversationRequest {
    #[validate(length(min = 1, max = 36))]
    character_id: String,
}

async fn list_conversations(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    AuthUser(user): AuthUser,
) -> ApiResult<Vec<ConversationListItemResponse>> {
    let rows = conversations::list_by_user(&state.db, &user.id).await?;
    Ok(ApiResponse::ok(
        rows.into_iter()
            .map(ConversationListItemResponse::from)
            .collect(),
    ))
}

async fn create_conversation(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    ValidatedJson(body): ValidatedJson<CreateConversationRequest>,
) -> ApiResult<ConversationResponse> {
    if let Some(conversation) =
        conversations::find_by_user_and_character(&state.db, &user.id, &body.character_id).await?
    {
        return Ok(ApiResponse::ok(conversation.into()));
    }

    characters::find_by_id(&state.db, &body.character_id)
        .await?
        .ok_or_else(|| AppError::not_found("角色不存在"))?;

    let conversation_id = Uuid::new_v4().to_string();
    match conversations::insert(
        &state.db,
        &conversation_id,
        &user.id,
        &body.character_id,
    )
    .await
    {
        Ok(conversation) => Ok(ApiResponse::created(conversation.into())),
        Err(err) => {
            if let Some(conversation) = conversations::find_by_user_and_character(
                &state.db,
                &user.id,
                &body.character_id,
            )
            .await?
            {
                return Ok(ApiResponse::ok(conversation.into()));
            }
            Err(err)
        }
    }
}
