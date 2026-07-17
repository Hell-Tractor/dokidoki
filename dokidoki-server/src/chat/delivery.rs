use std::sync::Arc;
use std::time::Duration;

use tokio::sync::watch;
use uuid::Uuid;

use crate::{config::Chat, db::queries::messages as message_queries, error::AppError};

use super::ChatService;

pub async fn deliver_staggered(
    chat: &Arc<ChatService>,
    user_id: &str,
    conversation_id: &str,
    turn_id: &str,
    reply_to_id: Option<&str>,
    bubbles: Vec<String>,
) -> Result<(), AppError> {
    if bubbles.is_empty() {
        return Ok(());
    }

    tracing::debug!(
        conversation_id,
        turn_id,
        bubbles = bubbles.len(),
        reply_to_id,
        "character delivery starting"
    );

    let (cancel_tx, cancel_rx) = watch::channel(false);
    {
        let mut deliveries = chat.active_deliveries.lock().await;
        deliveries.insert(
            conversation_id.to_owned(),
            super::ActiveDelivery {
                turn_id: turn_id.to_owned(),
                cancel: cancel_tx,
            },
        );
    }

    let result = deliver_loop(
        chat,
        user_id,
        conversation_id,
        turn_id,
        reply_to_id,
        bubbles,
        cancel_rx,
    )
    .await;

    chat.active_deliveries
        .lock()
        .await
        .remove(conversation_id);

    result
}

async fn deliver_loop(
    chat: &ChatService,
    user_id: &str,
    conversation_id: &str,
    turn_id: &str,
    reply_to_id: Option<&str>,
    bubbles: Vec<String>,
    mut cancel_rx: watch::Receiver<bool>,
) -> Result<(), AppError> {
    let total = bubbles.len();
    for (seq, content) in bubbles.into_iter().enumerate() {
        if *cancel_rx.borrow() {
            tracing::info!(
                conversation_id = %conversation_id,
                turn_id = %turn_id,
                delivered = seq,
                remaining = total - seq,
                "delivery cancelled before bubble"
            );
            chat.emit_turn_cancelled(user_id, conversation_id, turn_id)
                .await;
            return Ok(());
        }

        let message_id = Uuid::new_v4().to_string();
        let message = message_queries::insert_character_text(
            &chat.db,
            &message_id,
            conversation_id,
            &content,
            turn_id,
            seq as i32,
            reply_to_id,
        )
        .await?;

        chat.emit_message(user_id, conversation_id, message).await;

        if seq + 1 < total {
            let delay = bubble_delay_ms(&chat.chat_config, &content);
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_millis(delay)) => {},
                _ = cancel_rx.changed() => {
                    if *cancel_rx.borrow() {
                        tracing::info!(
                            conversation_id = %conversation_id,
                            turn_id = %turn_id,
                            delivered = seq + 1,
                            remaining = total - seq - 1,
                            "delivery cancelled during bubble delay"
                        );
                        chat.emit_turn_cancelled(user_id, conversation_id, turn_id)
                            .await;
                        return Ok(());
                    }
                }
            }
        }
    }

    tracing::info!(
        conversation_id = %conversation_id,
        turn_id = %turn_id,
        bubbles = total,
        "character delivery completed"
    );
    Ok(())
}

pub fn bubble_delay_ms(config: &Chat, content: &str) -> u64 {
    let chars = content.chars().count() as u64;
    config.bubble_delay_base_ms as u64 + chars * config.bubble_delay_per_char_ms as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bubble_delay_scales_with_length() {
        let config = Chat {
            burst_silence_ms: 1,
            bubble_delay_base_ms: 400,
            bubble_delay_per_char_ms: 50,
            reply_delay: crate::config::ReplyDelay::for_test(),
            winding_down_timeout_secs: 300,
            max_bubble_chars: 20,
            max_bubbles: 4,
            llm_format_retries: 2,
        };
        assert_eq!(bubble_delay_ms(&config, "你好"), 500);
        assert_eq!(bubble_delay_ms(&config, ""), 400);
    }
}
