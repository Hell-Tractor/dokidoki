use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use rand_core::{OsRng, RngCore};

use crate::{
    db::queries::{character_states, characters, conversations as conversation_queries},
    error::AppError,
};

use super::reply_delay::{activity_remaining_secs, compute_reply_wait_ms, ReplyDelayInput};
use super::ChatService;

pub async fn schedule(
    chat: &Arc<ChatService>,
    user_id: &str,
    conversation_id: &str,
    turn_id: &str,
    user_message_id: &str,
) -> Result<(), AppError> {
    let ctx = load_reply_context(chat, conversation_id).await?;
    let random_unit = (OsRng.next_u32() as f64) / (u32::MAX as f64);
    let delay = Duration::from_millis(compute_reply_wait_ms(&ctx, random_unit));
    // reply_wait 期间不显示 typing / 已读（已读由 M-17 在延迟窗口内调度）
    tokio::time::sleep(delay).await;

    chat.emit_character_typing(user_id, conversation_id, true).await;

    let bubbles = match chat
        .generate_character_bubbles(user_id, conversation_id, turn_id, user_message_id)
        .await
    {
        Ok(bubbles) => bubbles,
        Err(err) => {
            chat.emit_character_typing(user_id, conversation_id, false).await;
            return Err(err);
        }
    };

    if bubbles.is_empty() {
        chat.emit_character_typing(user_id, conversation_id, false).await;
        return Ok(());
    }

    super::delivery::deliver_staggered(
        chat,
        user_id,
        conversation_id,
        turn_id,
        Some(user_message_id),
        bubbles,
    )
    .await?;

    chat.emit_character_typing(user_id, conversation_id, false).await;
    Ok(())
}

async fn load_reply_context(
    chat: &ChatService,
    conversation_id: &str,
) -> Result<ReplyDelayInput, AppError> {
    let conversation = conversation_queries::find_by_id(&chat.db, conversation_id)
        .await?
        .ok_or_else(|| AppError::not_found("会话不存在"))?;

    let state = character_states::find_reply_fields(&chat.db, &conversation.character_id).await?;
    let persona = characters::find_persona_json(&chat.db, &conversation.character_id)
        .await?
        .unwrap_or_else(|| serde_json::json!({}));

    let availability = state
        .as_ref()
        .map(|row| row.availability.clone())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "medium".into());

    let activity_remaining_secs = activity_remaining_secs(
        state.and_then(|row| row.activity_ends_at),
        Utc::now(),
    );

    let proactive_tendency = persona
        .get("conversation_behavior")
        .and_then(|value| value.get("proactive_tendency"))
        .and_then(|value| value.as_str())
        .unwrap_or("normal")
        .to_owned();

    Ok(ReplyDelayInput {
        availability,
        proactive_tendency,
        activity_remaining_secs,
    })
}
