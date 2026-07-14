use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;

use crate::{
    db::queries::{character_states, characters, conversations as conversation_queries},
    error::AppError,
};

use super::read_receipt::sample_read_receipt_delay_ms;
use super::reply_delay::{activity_remaining_secs, compute_reply_wait_ms, ReplyDelayInput};
use super::ChatService;
use crate::utils::{OsUnitRng, UnitRng};

pub async fn schedule(
    chat: &Arc<ChatService>,
    user_id: &str,
    conversation_id: &str,
    turn_id: &str,
    user_message_ids: &[String],
) -> Result<(), AppError> {
    let ctx = load_reply_context(chat, conversation_id).await?;
    let mut rng = OsUnitRng;
    let reply_wait_ms =
        compute_reply_wait_ms(&ctx, &chat.chat_config.reply_delay, &mut rng);
    let read_delay_ms =
        sample_read_receipt_delay_ms(ctx.availability, reply_wait_ms, rng.next_unit());

    tokio::time::sleep(Duration::from_millis(read_delay_ms)).await;
    if let Err(err) = chat
        .mark_user_messages_read(user_id, conversation_id, user_message_ids)
        .await
    {
        tracing::warn!(
            conversation_id = %conversation_id,
            "delayed read receipt failed: {err}"
        );
    }

    let remaining_wait_ms = reply_wait_ms.saturating_sub(read_delay_ms);
    tokio::time::sleep(Duration::from_millis(remaining_wait_ms)).await;

    let Some(user_message_id) = user_message_ids.last() else {
        tracing::warn!(
            conversation_id = %conversation_id,
            "reply schedule skipped: empty user_message_ids"
        );
        return Ok(());
    };

    chat.emit_character_typing(user_id, conversation_id, true).await;

    let bubbles = match chat
        .generate_character_bubbles(
            user_id,
            conversation_id,
            turn_id,
            user_message_id,
        )
        .await
    {
        Ok(bubbles) => bubbles,
        Err(err) => {
            chat.emit_character_typing(user_id, conversation_id, false).await;
            return Err(err);
        }
    };

    if bubbles.is_empty() {
        tracing::debug!(
            conversation_id = %conversation_id,
            turn_id = %turn_id,
            "reply schedule: empty bubbles after generate"
        );
        chat.emit_character_typing(user_id, conversation_id, false).await;
        chat.spawn_maybe_compact(conversation_id);
        return Ok(());
    }

    super::delivery::deliver_staggered(
        chat,
        user_id,
        conversation_id,
        turn_id,
        user_message_ids.last().map(String::as_str),
        bubbles,
    )
    .await?;

    chat.emit_character_typing(user_id, conversation_id, false).await;
    chat.spawn_maybe_compact(conversation_id);
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
    let persona = characters::find_persona(&chat.db, &conversation.character_id)
        .await?
        .ok_or_else(|| AppError::not_found("角色不存在"))?;

    let availability = state
        .as_ref()
        .map(|row| row.availability)
        .unwrap_or(crate::domain::Availability::Medium);

    let activity_remaining_secs = activity_remaining_secs(
        state.and_then(|row| row.activity_ends_at),
        Utc::now(),
    );

    Ok(ReplyDelayInput {
        availability,
        factor_min: persona.reply_delay_factor.min,
        factor_max: persona.reply_delay_factor.max,
        activity_remaining_secs,
    })
}
