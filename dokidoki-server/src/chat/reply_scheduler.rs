use std::sync::Arc;
use std::time::Duration;

use rand_core::{OsRng, RngCore};

use crate::{config::Chat, error::AppError};

use super::ChatService;

pub async fn schedule(
    chat: &Arc<ChatService>,
    user_id: &str,
    conversation_id: &str,
    turn_id: &str,
    user_message_id: &str,
) -> Result<(), AppError> {
    let delay = Duration::from_millis(sample_reply_delay_ms(&chat.chat_config));
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

pub fn sample_reply_delay_ms(config: &Chat) -> u64 {
    let min = config.min_reply_delay_ms as u64;
    let max = config.max_reply_delay_ms.max(config.min_reply_delay_ms) as u64;
    if min >= max {
        return min;
    }
    min + (OsRng.next_u32() as u64 % (max - min + 1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Chat;

    fn test_chat_config(min: u32, max: u32) -> Chat {
        Chat {
            burst_silence_ms: 1,
            min_reply_delay_ms: min,
            max_reply_delay_ms: max,
            bubble_delay_base_ms: 1,
            bubble_delay_per_char_ms: 1,
        }
    }

    #[test]
    fn sample_reply_delay_within_bounds() {
        let config = test_chat_config(300, 800);
        for _ in 0..100 {
            let delay = sample_reply_delay_ms(&config);
            assert!(delay >= 300);
            assert!(delay <= 800);
        }
    }
}
