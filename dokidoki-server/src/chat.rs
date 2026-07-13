use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use sqlx::MySqlPool;
use tokio::sync::{watch, Mutex};
use uuid::Uuid;

use crate::{
    config::Chat,
    db::queries::{characters, conversations as conversation_queries, messages as message_queries},
    db::message::Message,
    domain::messages::SentTextMessage,
    error::AppError,
    llm::LlmClient,
    ws_hub::WsHub,
};

mod burst;
mod context;
mod delivery;
pub mod conversation_fsm;
pub mod parser;
mod reply_scheduler;

use conversation_fsm::{
    on_user_message, status_after_llm_action, ConversationStatus, UserMessageDecision,
};
use parser::{parse_action, LlmAction};

pub(super) struct ActiveDelivery {
    turn_id: String,
    cancel: watch::Sender<bool>,
}

pub struct ChatService {
    pub(super) db: MySqlPool,
    pub(super) llm: Arc<LlmClient>,
    pub(super) ws_hub: Arc<WsHub>,
    pub(super) chat_config: Chat,
    pub(super) burst_buffers: Arc<Mutex<HashMap<String, burst::BurstBuffer>>>,
    pub(super) active_deliveries: Arc<Mutex<HashMap<String, ActiveDelivery>>>,
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

#[derive(Serialize)]
struct WsTypingPayload {
    conversation_id: String,
    active: bool,
}

#[derive(Serialize)]
struct WsTurnCancelledPayload {
    conversation_id: String,
    turn_id: String,
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
    pub fn new(
        db: MySqlPool,
        llm: Arc<LlmClient>,
        ws_hub: Arc<WsHub>,
        chat_config: Chat,
    ) -> Self {
        Self {
            db,
            llm,
            ws_hub,
            chat_config,
            burst_buffers: Arc::new(Mutex::new(HashMap::new())),
            active_deliveries: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn ingest_user_text(
        self: &Arc<Self>,
        user_id: &str,
        conversation_id: &str,
        content: String,
    ) -> Result<SentTextMessage, AppError> {
        burst::ingest_user_text(self, user_id, conversation_id, content).await
    }

    pub(super) async fn flush_burst(
        self: &Arc<Self>,
        user_id: &str,
        conversation_id: &str,
    ) -> Result<(), AppError> {
        let burst = {
            let mut buffers = self.burst_buffers.lock().await;
            buffers.remove(conversation_id)
        };

        let Some(burst) = burst else {
            return Ok(());
        };

        burst.timer.abort();

        reply_scheduler::schedule(
            self,
            user_id,
            conversation_id,
            &burst.turn_id,
            &burst.last_message_id,
        )
        .await
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

    async fn run_icebreaker(self: &Arc<Self>, user_id: &str, conversation_id: &str) -> Result<(), AppError> {
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

        self.emit_character_typing(user_id, conversation_id, true).await;
        delivery::deliver_staggered(self, user_id, conversation_id, &turn_id, None, bubbles).await?;
        self.emit_character_typing(user_id, conversation_id, false).await;
        Ok(())
    }

    pub(super) async fn generate_character_bubbles(
        &self,
        user_id: &str,
        conversation_id: &str,
        turn_id: &str,
        user_message_id: &str,
    ) -> Result<Vec<String>, AppError> {
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
                return Ok(Vec::new());
            }
            UserMessageDecision::IgnoreWhilePaused => return Ok(Vec::new()),
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

        Ok(match action {
            LlmAction::NoReply => Vec::new(),
            LlmAction::Reply(bubbles) | LlmAction::EndTopic(bubbles) => bubbles,
        })
    }

    pub(super) async fn cancel_active_delivery(&self, user_id: &str, conversation_id: &str) {
        let delivery = {
            let mut deliveries = self.active_deliveries.lock().await;
            deliveries.remove(conversation_id)
        };

        let Some(delivery) = delivery else {
            return;
        };

        let _ = delivery.cancel.send(true);
        self.emit_turn_cancelled(user_id, conversation_id, &delivery.turn_id)
            .await;
    }

    pub(super) async fn emit_message(
        &self,
        user_id: &str,
        conversation_id: &str,
        message: Message,
    ) {
        let payload = WsMessagePayload::from((message, conversation_id.to_owned()));
        self.ws_hub
            .emit_json(user_id, conversation_id, "message", &payload)
            .await;
    }

    pub(super) async fn emit_character_typing(
        &self,
        user_id: &str,
        conversation_id: &str,
        active: bool,
    ) {
        self.ws_hub
            .emit_json(
                user_id,
                conversation_id,
                "character_typing",
                WsTypingPayload {
                    conversation_id: conversation_id.to_owned(),
                    active,
                },
            )
            .await;
    }

    pub(super) async fn emit_turn_cancelled(
        &self,
        user_id: &str,
        conversation_id: &str,
        turn_id: &str,
    ) {
        self.ws_hub
            .emit_json(
                user_id,
                conversation_id,
                "turn_cancelled",
                WsTurnCancelledPayload {
                    conversation_id: conversation_id.to_owned(),
                    turn_id: turn_id.to_owned(),
                },
            )
            .await;
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
}

fn proactive_tendency(persona: &Value) -> &str {
    persona
        .get("conversation_behavior")
        .and_then(|value| value.get("proactive_tendency"))
        .and_then(Value::as_str)
        .unwrap_or("normal")
}
