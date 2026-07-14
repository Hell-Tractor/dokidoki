use std::sync::Arc;

use axum::{routing::get, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    api::{
        extractors::{AuthUser, ValidatedJson},
        response::{ApiResponse, ApiResult},
    },
    domain::conversations::{
        self, ConversationListItem, GetOrCreateConversationResult, LastMessagePreview,
    },
    db::models::Conversation,
    state::AppState,
};

pub fn api() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_conversations).post(create_conversation))
        .nest("/{conversation_id}/messages", super::messages::api())
}

#[derive(Serialize)]
struct ConversationResponse {
    id: String,
    character_id: String,
    status: String,
    first_contact_done: bool,
}

impl From<Conversation> for ConversationResponse {
    fn from(conversation: Conversation) -> Self {
        Self {
            id: conversation.id,
            character_id: conversation.character_id,
            status: conversation.status.as_str().to_owned(),
            first_contact_done: conversation.first_contact_done,
        }
    }
}

#[derive(Serialize)]
struct LastMessageResponse {
    content: String,
    created_at: DateTime<Utc>,
    role: String,
}

impl From<LastMessagePreview> for LastMessageResponse {
    fn from(preview: LastMessagePreview) -> Self {
        Self {
            content: preview.content,
            created_at: preview.created_at,
            role: preview.role,
        }
    }
}

#[derive(Serialize)]
struct ConversationListItemResponse {
    id: String,
    character_id: String,
    character_name: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_message: Option<LastMessageResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    current_activity: Option<String>,
}

impl From<ConversationListItem> for ConversationListItemResponse {
    fn from(item: ConversationListItem) -> Self {
        Self {
            id: item.id,
            character_id: item.character_id,
            character_name: item.character_name,
            status: item.status,
            last_message: item.last_message.map(LastMessageResponse::from),
            current_activity: item.current_activity,
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
    let items = conversations::list_for_user(&state.db, &user.id).await?;
    Ok(ApiResponse::ok(
        items
            .into_iter()
            .map(ConversationListItemResponse::from)
            .collect(),
    ))
}

async fn create_conversation(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    ValidatedJson(body): ValidatedJson<CreateConversationRequest>,
) -> ApiResult<ConversationResponse> {
    let result = conversations::get_or_create(&state.db, &user.id, &body.character_id).await?;
    let conversation = match &result {
        GetOrCreateConversationResult::Created(conversation) => conversation,
        GetOrCreateConversationResult::Existing(conversation) => conversation,
    };

    if !conversation.first_contact_done {
        state
            .chat
            .maybe_trigger_icebreaker(&user.id, &conversation.id);
    }

    match result {
        GetOrCreateConversationResult::Created(conversation) => {
            Ok(ApiResponse::created(conversation.into()))
        }
        GetOrCreateConversationResult::Existing(conversation) => {
            Ok(ApiResponse::ok(conversation.into()))
        }
    }
}
