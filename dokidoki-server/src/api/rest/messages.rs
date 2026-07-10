use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    routing::get,
    Router,
};
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
        message::{Message, CONTENT_TYPE_IMAGE},
        queries::{conversations, messages},
    },
    error::AppError,
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
pub enum MessageResponse {
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

impl From<Message> for MessageResponse {
    fn from(message: Message) -> Self {
        if message.content_type == CONTENT_TYPE_IMAGE {
            let image_url = message
                .media_url("image")
                .unwrap_or_else(|| format!("/api/v1/messages/{}/image", message.id));
            MessageResponse::Image {
                id: message.id,
                role: message.role,
                content: message.content.unwrap_or_default(),
                image_url,
                turn_id: message.turn_id,
                seq_in_turn: message.seq_in_turn,
                reply_to_id: message.reply_to_id,
                read_at: message.read_at,
                created_at: message.created_at,
            }
        } else {
            MessageResponse::Text {
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
    }
}

#[derive(Serialize)]
pub struct ListMessagesResponse {
    pub messages: Vec<MessageResponse>,
    pub has_more: bool,
}

#[derive(Serialize)]
pub struct CreateMessageResponse {
    pub id: String,
    pub turn_id: String,
    pub created_at: DateTime<Utc>,
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

    conversations::find_by_id_for_user(&state.db, &conversation_id, &user.id)
        .await?
        .ok_or_else(|| AppError::not_found("会话不存在"))?;

    let (rows, has_more) =
        messages::list_page(&state.db, &conversation_id, query.before.as_deref(), limit).await?;

    Ok(ApiResponse::ok(ListMessagesResponse {
        messages: rows.into_iter().map(MessageResponse::from).collect(),
        has_more,
    }))
}

async fn create_message(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Path(conversation_id): Path<String>,
    ValidatedJson(body): ValidatedJson<CreateMessageRequest>,
) -> ApiResult<CreateMessageResponse> {
    conversations::find_by_id_for_user(&state.db, &conversation_id, &user.id)
        .await?
        .ok_or_else(|| AppError::not_found("会话不存在"))?;

    let content = body.content.trim();
    if content.is_empty() {
        return Err(AppError::bad_request("消息内容不能为空"));
    }

    let message_id = Uuid::new_v4().to_string();
    let turn_id = Uuid::new_v4().to_string();
    let message = messages::insert_user_text(
        &state.db,
        &message_id,
        &conversation_id,
        content,
        &turn_id,
    )
    .await?;

    Ok(ApiResponse::accepted(CreateMessageResponse {
        id: message.id,
        turn_id: message.turn_id.unwrap_or(turn_id),
        created_at: message.created_at,
    }))
}
