use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::{
    db::queries::{characters, conversations as conversation_queries, messages as message_queries},
    db::message::Message,
    error::AppError,
    llm::LlmClient,
    ws_hub::WsHub,
};

mod context;
pub mod conversation_fsm;
pub mod parser;

use conversation_fsm::{
    on_user_message, status_after_llm_action, ConversationStatus, UserMessageDecision,
};
use parser::{parse_action, LlmAction};

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

    pub fn maybe_trigger_icebreaker(self: &Arc<Self>, user_id: &str, conversation_id: &str) {
        let this = Arc::clone(self);
        let user_id = user_id.to_owned();
        let conversation_id = conversation_id.to_owned();

        tokio::spawn(async move {
            if let Err(err) = this.run_icebreaker(&user_id, &conversation_id).await {
                tracing::error!(
                    conversation_id = %conversation_id,
                    "icebreaker failed: {err}"
                );
            }
        });
    }

    async fn run_icebreaker(&self, user_id: &str, conversation_id: &str) -> Result<(), AppError> {
        if !conversation_queries::try_begin_icebreaker(&self.db, conversation_id).await? {
            return Ok(());
        }

        let turn_id = Uuid::new_v4().to_string();
        let request =
            context::build_icebreaker_request(&self.db, user_id, conversation_id, &turn_id).await?;
        let raw = self.llm.chat(request).await?;
        let bubbles = parser::parse_reply(&raw);

        if bubbles.is_empty() {
            conversation_queries::rollback_icebreaker(&self.db, conversation_id).await?;
            return Ok(());
        }

        self.deliver_character_bubbles(user_id, conversation_id, &turn_id, None, bubbles)
            .await
    }

    async fn process_turn(
        &self,
        user_id: &str,
        conversation_id: &str,
        turn_id: &str,
        user_message_id: &str,
    ) -> Result<(), AppError> {
        let conversation = conversation_queries::find_by_id(&self.db, conversation_id)
            .await?
            .ok_or_else(|| AppError::not_found("会话不存在"))?;

        if conversation.user_id != user_id {
            return Err(AppError::not_found("会话不存在"));
        }

        let user_message =
            message_queries::find_by_id_in_conversation(&self.db, conversation_id, user_message_id)
                .await?
                .ok_or_else(|| AppError::not_found("消息不存在"))?;

        let user_content = user_message.content.unwrap_or_default();
        let persona_json = characters::find_persona_json(&self.db, &conversation.character_id)
            .await?
            .unwrap_or_else(|| Value::Object(Default::default()));
        let proactive_tendency = proactive_tendency(&persona_json);

        let current_status = ConversationStatus::parse(&conversation.status)
            .unwrap_or(ConversationStatus::Active);

        match on_user_message(current_status, &user_content, proactive_tendency) {
            UserMessageDecision::PauseWithoutReply => {
                self.update_conversation_status(conversation_id, ConversationStatus::Paused)
                    .await?;
                return Ok(());
            }
            UserMessageDecision::IgnoreWhilePaused => return Ok(()),
            UserMessageDecision::CallLlm { status } => {
                if let Some(status) = status {
                    self.update_conversation_status(conversation_id, status)
                        .await?;
                }
            }
        }

        let request =
            context::build_chat_request(&self.db, user_id, conversation_id, turn_id).await?;
        let raw = self.llm.chat(request).await?;
        let action = parse_action(&raw);

        if let Some(status) = status_after_llm_action(action.clone()) {
            self.update_conversation_status(conversation_id, status)
                .await?;
        }

        let bubbles = match action {
            LlmAction::NoReply => return Ok(()),
            LlmAction::Reply(bubbles) | LlmAction::EndTopic(bubbles) => bubbles,
        };

        if bubbles.is_empty() {
            return Ok(());
        }

        self.deliver_character_bubbles(
            user_id,
            conversation_id,
            turn_id,
            Some(user_message_id),
            bubbles,
        )
        .await
    }

    async fn update_conversation_status(
        &self,
        conversation_id: &str,
        status: ConversationStatus,
    ) -> Result<(), AppError> {
        let set_paused_at = status == ConversationStatus::Paused;
        conversation_queries::update_status(
            &self.db,
            conversation_id,
            status.as_str(),
            set_paused_at,
        )
        .await
    }

    async fn deliver_character_bubbles(
        &self,
        user_id: &str,
        conversation_id: &str,
        turn_id: &str,
        reply_to_id: Option<&str>,
        bubbles: Vec<String>,
    ) -> Result<(), AppError> {
        for (seq, content) in bubbles.into_iter().enumerate() {
            let message_id = Uuid::new_v4().to_string();
            let message = message_queries::insert_character_text(
                &self.db,
                &message_id,
                conversation_id,
                &content,
                turn_id,
                seq as i32,
                reply_to_id,
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

fn proactive_tendency(persona: &Value) -> &str {
    persona
        .get("conversation_behavior")
        .and_then(|value| value.get("proactive_tendency"))
        .and_then(Value::as_str)
        .unwrap_or("normal")
}
