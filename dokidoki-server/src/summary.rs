use std::sync::Arc;

use crate::{chat::ChatService, error::AppError};

mod compact;

pub use compact::{
    format_messages_for_summary, select_turns_to_compact, should_compact, truncate_summary,
    TurnInfo,
};

pub async fn maybe_compact(chat: &Arc<ChatService>, conversation_id: &str) -> Result<(), AppError> {
    {
        let mut in_progress = chat.compacting.lock().await;
        if !in_progress.insert(conversation_id.to_owned()) {
            return Ok(());
        }
    }

    let result = compact::run_compact(
        &chat.db,
        &chat.llm,
        conversation_id,
        &chat.summary_config,
    )
    .await;

    chat.compacting.lock().await.remove(conversation_id);
    result
}
