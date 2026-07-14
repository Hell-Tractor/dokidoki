use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::MySqlPool;
use tokio::sync::{watch, Mutex};
use uuid::Uuid;

use crate::{
    config::{Chat, Proactive, Summary},
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
mod read_receipt;
mod reply_delay;
mod reply_scheduler;

pub use context::build_proactive_request;

use conversation_fsm::{
    on_user_message, status_after_llm_action, ConversationStatus, UserMessageDecision,
};
use parser::LlmAction;

pub(super) struct ActiveDelivery {
    turn_id: String,
    cancel: watch::Sender<bool>,
}

pub struct ChatService {
    pub(crate) db: MySqlPool,
    pub(crate) llm: Arc<LlmClient>,
    pub(crate) ws_hub: Arc<WsHub>,
    pub(crate) chat_config: Chat,
    pub(crate) summary_config: Summary,
    pub(crate) proactive_config: Proactive,
    burst_buffers: Arc<Mutex<HashMap<String, burst::BurstBuffer>>>,
    pub(crate) active_deliveries: Arc<Mutex<HashMap<String, ActiveDelivery>>>,
    pub(crate) compacting: Arc<Mutex<HashSet<String>>>,
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
struct WsMessageReadPayload {
    conversation_id: String,
    message_ids: Vec<String>,
    read_at: DateTime<Utc>,
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
        summary_config: Summary,
        proactive_config: Proactive,
    ) -> Self {
        Self {
            db,
            llm,
            ws_hub,
            chat_config,
            summary_config,
            proactive_config,
            burst_buffers: Arc::new(Mutex::new(HashMap::new())),
            active_deliveries: Arc::new(Mutex::new(HashMap::new())),
            compacting: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub fn spawn_maybe_compact(self: &Arc<Self>, conversation_id: &str) {
        let chat = Arc::clone(self);
        let conversation_id = conversation_id.to_owned();
        tokio::spawn(async move {
            if let Err(err) = crate::summary::maybe_compact(&chat, &conversation_id).await {
                tracing::warn!(
                    conversation_id = %conversation_id,
                    "summary compact failed: {err}"
                );
            }
        });
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

        reply_scheduler::schedule(
            self,
            user_id,
            conversation_id,
            &burst.turn_id,
            &burst.message_ids,
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
            tracing::debug!(
                conversation_id = %conversation_id,
                "icebreaker skipped: already started or done"
            );
            return Ok(());
        }

        let turn_id = Uuid::new_v4().to_string();
        let request =
            context::build_icebreaker_request(&self.db, user_id, conversation_id, &turn_id).await?;
        let raw = self.llm.chat(request).await?;
        let bubbles = parser::parse_reply(&raw);

        if bubbles.is_empty() {
            tracing::warn!(
                conversation_id = %conversation_id,
                raw_len = raw.len(),
                "icebreaker empty reply; rolling back first_contact_done"
            );
            conversation_queries::rollback_icebreaker(&self.db, conversation_id).await?;
            return Ok(());
        }

        tracing::info!(
            conversation_id = %conversation_id,
            bubbles = bubbles.len(),
            "icebreaker delivering"
        );
        self.emit_character_typing(user_id, conversation_id, true).await;
        delivery::deliver_staggered(self, user_id, conversation_id, &turn_id, None, bubbles).await?;
        self.emit_character_typing(user_id, conversation_id, false).await;
        self.spawn_maybe_compact(conversation_id);
        Ok(())
    }

    pub(crate) async fn has_active_delivery(&self, conversation_id: &str) -> bool {
        self.active_deliveries.lock().await.contains_key(conversation_id)
    }

    /// 主动消息气泡投递（无 reply_to）；成功后触发摘要压缩。
    pub(crate) async fn deliver_proactive_bubbles(
        self: &Arc<Self>,
        user_id: &str,
        conversation_id: &str,
        turn_id: &str,
        bubbles: Vec<String>,
    ) -> Result<(), AppError> {
        self.emit_character_typing(user_id, conversation_id, true).await;
        let result =
            delivery::deliver_staggered(self, user_id, conversation_id, turn_id, None, bubbles)
                .await;
        self.emit_character_typing(user_id, conversation_id, false)
            .await;
        result?;
        self.spawn_maybe_compact(conversation_id);
        Ok(())
    }

    pub(crate) async fn generate_character_bubbles(
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
        let persona = characters::find_persona(&self.db, &conversation.character_id)
            .await?
            .ok_or_else(|| AppError::not_found("角色不存在"))?;
        let pause_on_farewell = persona.conversation_behavior.pause_on_farewell;

        let current_status = ConversationStatus::parse(&conversation.status)
            .unwrap_or(ConversationStatus::Active);

        match on_user_message(current_status, &user_content, pause_on_farewell) {
            UserMessageDecision::PauseWithoutReply => {
                tracing::info!(
                    conversation_id = %conversation_id,
                    "farewell in winding_down; pause without reply"
                );
                self.update_conversation_status(conversation_id, ConversationStatus::Paused)
                    .await?;
                return Ok(Vec::new());
            }
            UserMessageDecision::IgnoreWhilePaused => {
                tracing::debug!(
                    conversation_id = %conversation_id,
                    "ignore user message while paused"
                );
                return Ok(Vec::new());
            }
            UserMessageDecision::CallLlm { status } => {
                if let Some(status) = status {
                    tracing::debug!(
                        conversation_id = %conversation_id,
                        status = status.as_str(),
                        "conversation status updated before llm"
                    );
                    self.update_conversation_status(conversation_id, status)
                        .await?;
                }
            }
        }

        let request = context::build_chat_request(
            &self.db,
            user_id,
            conversation_id,
            turn_id,
            self.summary_config.keep_recent_turns,
        )
        .await?;
        let raw = self.llm.chat(request).await?;
        let parsed = crate::memory::parse_llm_response(&raw);
        crate::memory::apply_side_effects(
            &self.db,
            user_id,
            &conversation.character_id,
            &parsed,
        )
        .await?;

        if let Some(status) = status_after_llm_action(parsed.action.clone()) {
            tracing::info!(
                conversation_id = %conversation_id,
                status = status.as_str(),
                "conversation status updated after llm action"
            );
            self.update_conversation_status(conversation_id, status)
                .await?;
        }

        Ok(match parsed.action {
            LlmAction::NoReply => {
                tracing::info!(
                    conversation_id = %conversation_id,
                    "llm returned NO_REPLY"
                );
                Vec::new()
            }
            LlmAction::Reply(bubbles) => {
                tracing::debug!(
                    conversation_id = %conversation_id,
                    bubbles = bubbles.len(),
                    "llm returned REPLY"
                );
                bubbles
            }
            LlmAction::EndTopic(bubbles) => {
                tracing::info!(
                    conversation_id = %conversation_id,
                    bubbles = bubbles.len(),
                    "llm returned END_TOPIC"
                );
                bubbles
            }
        })
    }

    pub(crate) async fn cancel_active_delivery(&self, user_id: &str, conversation_id: &str) {
        let delivery = {
            let mut deliveries = self.active_deliveries.lock().await;
            deliveries.remove(conversation_id)
        };

        let Some(delivery) = delivery else {
            return;
        };

        tracing::info!(
            conversation_id = %conversation_id,
            turn_id = %delivery.turn_id,
            "cancelling active delivery"
        );
        if delivery.cancel.send(true).is_err() {
            tracing::debug!(
                conversation_id = %conversation_id,
                turn_id = %delivery.turn_id,
                "delivery cancel signal dropped (receiver gone)"
            );
        }
        self.emit_turn_cancelled(user_id, conversation_id, &delivery.turn_id)
            .await;
    }

    pub(crate) async fn emit_message(
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

    pub(crate) async fn emit_character_typing(
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

    pub(crate) async fn emit_message_read(
        &self,
        user_id: &str,
        conversation_id: &str,
        message_ids: &[String],
        read_at: DateTime<Utc>,
    ) {
        if message_ids.is_empty() {
            return;
        }

        self.ws_hub
            .emit_json(
                user_id,
                conversation_id,
                "message_read",
                WsMessageReadPayload {
                    conversation_id: conversation_id.to_owned(),
                    message_ids: message_ids.to_vec(),
                    read_at,
                },
            )
            .await;
    }

    pub(crate) async fn mark_user_messages_read(
        &self,
        user_id: &str,
        conversation_id: &str,
        message_ids: &[String],
    ) -> Result<Option<DateTime<Utc>>, AppError> {
        let read_at = message_queries::mark_user_messages_read(
            &self.db,
            conversation_id,
            user_id,
            message_ids,
        )
        .await?;

        let Some(read_at) = read_at else {
            return Ok(None);
        };

        self.emit_message_read(user_id, conversation_id, message_ids, read_at)
            .await;
        Ok(Some(read_at))
    }

    pub(crate) async fn emit_turn_cancelled(
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
