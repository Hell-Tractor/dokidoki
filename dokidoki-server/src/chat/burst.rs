use std::sync::Arc;
use std::time::Duration;

use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::{
    db::queries::messages as message_queries,
    domain::conversations,
    domain::messages::SentTextMessage,
    error::AppError,
};

use super::ChatService;

pub(super) struct BurstBuffer {
    pub(super) turn_id: String,
    pub(super) message_ids: Vec<String>,
    pub(super) last_message_id: String,
    pub(super) timer: JoinHandle<()>,
}

pub async fn ingest_user_text(
    chat: &Arc<ChatService>,
    user_id: &str,
    conversation_id: &str,
    content: String,
) -> Result<SentTextMessage, AppError> {
    conversations::require_owned(&chat.db, conversation_id, user_id).await?;

    let content = content.trim();
    if content.is_empty() {
        return Err(AppError::bad_request("消息内容不能为空"));
    }

    chat.cancel_active_delivery(user_id, conversation_id).await;

    let message_id = Uuid::new_v4().to_string();
    let (turn_id, seq_in_turn, is_append) = {
        let mut buffers = chat.burst_buffers.lock().await;
        if let Some(existing) = buffers.get_mut(conversation_id) {
            existing.timer.abort();
            let turn_id = existing.turn_id.clone();
            let seq = existing.message_ids.len() as i32;
            existing.message_ids.push(message_id.clone());
            existing.last_message_id = message_id.clone();
            (turn_id, seq, true)
        } else {
            (Uuid::new_v4().to_string(), 0, false)
        }
    };

    let message = message_queries::insert_user_burst_text(
        &chat.db,
        &message_id,
        conversation_id,
        content,
        &turn_id,
        seq_in_turn,
    )
    .await?;

    tracing::debug!(
        conversation_id,
        turn_id = %turn_id,
        message_id = %message.id,
        seq_in_turn,
        is_append,
        content_chars = content.chars().count(),
        "user burst text ingested"
    );

    let silence = Duration::from_millis(chat.chat_config.burst_silence_ms as u64);
    let this = Arc::clone(chat);
    let user_id_owned = user_id.to_owned();
    let conversation_id_owned = conversation_id.to_owned();
    let timer = tokio::spawn(async move {
        tokio::time::sleep(silence).await;
        if let Err(err) = this
            .flush_burst(&user_id_owned, &conversation_id_owned)
            .await
        {
            tracing::error!(
                conversation_id = %conversation_id_owned,
                "burst flush failed: {err}"
            );
        }
    });

    {
        let mut buffers = chat.burst_buffers.lock().await;
        match buffers.get_mut(conversation_id) {
            Some(existing) => {
                existing.timer = timer;
            }
            None => {
                buffers.insert(
                    conversation_id.to_owned(),
                    BurstBuffer {
                        turn_id: turn_id.clone(),
                        message_ids: vec![message_id.clone()],
                        last_message_id: message_id.clone(),
                        timer,
                    },
                );
            }
        }
    }

    Ok(SentTextMessage {
        id: message.id,
        turn_id: message.turn_id.unwrap_or(turn_id),
        created_at: message.created_at,
    })
}
