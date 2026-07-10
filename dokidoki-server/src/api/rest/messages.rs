use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    routing::get,
    Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    api::{
        extractors::{AuthUser, ValidatedJson},
        response::{ApiResponse, ApiResult},
    },
    db::message::{Message, CONTENT_TYPE_IMAGE, CONTENT_TYPE_TEXT},
    domain::messages::{self, SentTextMessage},
    state::AppState,
};

pub fn api() -> Router<Arc<AppState>> {
    Router::new().route("/", get(list_messages).post(create_message))
}

#[derive(Deserialize)]
struct ListMessagesQuery {
    before: Option<String>,
    #[serde(default = "default_limit")]
    limit: u32,
}

fn default_limit() -> u32 {
    50
}

#[derive(Serialize)]
#[serde(tag = "content_type", rename_all = "snake_case")]
enum MessageResponse {
    Text {
        id: String,
        role: String,
        content: String,
        turn_id: Option<String>,
        seq_in_turn: i32,
        reply_to_id: Option<String>,
        read_at: Option<DateTime<Utc>>,
        created_at: DateTime<Utc>,
    },
    Image {
        id: String,
        role: String,
        content: String,
        image_url: String,
        turn_id: Option<String>,
        seq_in_turn: i32,
        reply_to_id: Option<String>,
        read_at: Option<DateTime<Utc>>,
        created_at: DateTime<Utc>,
    },
}

struct MessageCommonFields {
    id: String,
    role: String,
    content: String,
    turn_id: Option<String>,
    seq_in_turn: i32,
    reply_to_id: Option<String>,
    read_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl MessageCommonFields {
    fn from_message(message: Message) -> Self {
        Self {
            id: message.id,
            role: message.role,
            content: message.content.unwrap_or_default(),
            turn_id: message.turn_id,
            seq_in_turn: message.seq_in_turn,
            reply_to_id: message.reply_to_id,
            read_at: message.read_at,
            created_at: message.created_at,
        }
    }

    fn into_text(self) -> MessageResponse {
        MessageResponse::Text {
            id: self.id,
            role: self.role,
            content: self.content,
            turn_id: self.turn_id,
            seq_in_turn: self.seq_in_turn,
            reply_to_id: self.reply_to_id,
            read_at: self.read_at,
            created_at: self.created_at,
        }
    }

    fn into_image(self, image_url: String) -> MessageResponse {
        MessageResponse::Image {
            id: self.id,
            role: self.role,
            content: self.content,
            image_url,
            turn_id: self.turn_id,
            seq_in_turn: self.seq_in_turn,
            reply_to_id: self.reply_to_id,
            read_at: self.read_at,
            created_at: self.created_at,
        }
    }
}

impl From<Message> for MessageResponse {
    fn from(message: Message) -> Self {
        let content_type = message.content_type.clone();
        let image_url = message.media_url("image");
        let common = MessageCommonFields::from_message(message);

        if content_type == CONTENT_TYPE_IMAGE {
            if let Some(image_url) = image_url {
                return common.into_image(image_url);
            }
        }

        let _ = CONTENT_TYPE_TEXT;
        common.into_text()
    }
}

#[derive(Serialize)]
struct ListMessagesResponse {
    messages: Vec<MessageResponse>,
    has_more: bool,
}

#[derive(Serialize)]
struct CreateMessageResponse {
    id: String,
    turn_id: String,
    created_at: DateTime<Utc>,
}

impl From<SentTextMessage> for CreateMessageResponse {
    fn from(message: SentTextMessage) -> Self {
        Self {
            id: message.id,
            turn_id: message.turn_id,
            created_at: message.created_at,
        }
    }
}

#[derive(Deserialize, Validate)]
struct CreateMessageRequest {
    #[validate(length(min = 1, max = 10000))]
    content: String,
}

async fn list_messages(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Path(conversation_id): Path<String>,
    Query(query): Query<ListMessagesQuery>,
) -> ApiResult<ListMessagesResponse> {
    let limit = query.limit.clamp(1, 100);
    let page = messages::list_for_conversation(
        &state.db,
        &user.id,
        &conversation_id,
        query.before.as_deref(),
        limit,
    )
    .await?;

    Ok(ApiResponse::ok(ListMessagesResponse {
        messages: page
            .messages
            .into_iter()
            .map(MessageResponse::from)
            .collect(),
        has_more: page.has_more,
    }))
}

async fn create_message(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Path(conversation_id): Path<String>,
    ValidatedJson(body): ValidatedJson<CreateMessageRequest>,
) -> ApiResult<CreateMessageResponse> {
    let message = messages::send_user_text(
        &state.db,
        &user.id,
        &conversation_id,
        body.content,
    )
    .await?;

    Ok(ApiResponse::accepted(message.into()))
}
