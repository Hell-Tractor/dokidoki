use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::{
    db::{message::Message, queries::messages as message_queries},
    error::AppError,
    llm::{ChatRequest, LlmClient},
    ws_hub::WsHub,
};

pub mod parser;

pub struct ChatService {
    db: MySqlPool,
    llm: Arc<LlmClient>,
    ws_hub: Arc<WsHub>,
}

#[derive(Serialize)]
struct WsMessagePayload {
    id: String,
    conversation_id: String,
    role: String,
    content: String,
    content_type: String,
    turn_id: String,
    seq_in_turn: i32,
    created_at: DateTime<Utc>,
}

impl From<(Message, String)> for WsMessagePayload {
    fn from((message, conversation_id): (Message, String)) -> Self {
        Self {
            id: message.id,
            conversation_id,
            role: message.role,
            content: message.content.unwrap_or_default(),
            content_type: message.content_type,
            turn_id: message.turn_id.unwrap_or_default(),
            seq_in_turn: message.seq_in_turn,
            created_at: message.created_at,
        }
    }
}

impl ChatService {
    pub fn new(db: MySqlPool, llm: Arc<LlmClient>, ws_hub: Arc<WsHub>) -> Self {
        Self { db, llm, ws_hub }
    }

    pub fn on_user_text_sent(
        self: &Arc<Self>,
        user_id: &str,
        conversation_id: &str,
        turn_id: &str,
        user_message_id: &str,
    ) {
        let this = Arc::clone(self);
        let user_id = user_id.to_owned();
        let conversation_id = conversation_id.to_owned();
        let turn_id = turn_id.to_owned();
        let user_message_id = user_message_id.to_owned();

        tokio::spawn(async move {
            if let Err(err) = this
                .process_turn(&user_id, &conversation_id, &turn_id, &user_message_id)
                .await
            {
                tracing::error!(
                    conversation_id = %conversation_id,
                    turn_id = %turn_id,
                    "character reply failed: {err}"
                );
            }
        });
    }

    async fn process_turn(
        &self,
        user_id: &str,
        conversation_id: &str,
        turn_id: &str,
        user_message_id: &str,
    ) -> Result<(), AppError> {
        let raw = self
            .llm
            .chat(ChatRequest {
                conversation_id: conversation_id.to_owned(),
                turn_id: turn_id.to_owned(),
            })
            .await?;

        let bubbles = parser::parse_reply(&raw);
        if bubbles.is_empty() {
            return Ok(());
        }

        for (seq, content) in bubbles.into_iter().enumerate() {
            let message_id = Uuid::new_v4().to_string();
            let message = message_queries::insert_character_text(
                &self.db,
                &message_id,
                conversation_id,
                &content,
                turn_id,
                seq as i32,
                Some(user_message_id),
            )
            .await?;

            let payload = WsMessagePayload::from((message, conversation_id.to_owned()));
            self.ws_hub
                .emit_json(user_id, conversation_id, "message", &payload)
                .await;
        }

        Ok(())
    }
}
