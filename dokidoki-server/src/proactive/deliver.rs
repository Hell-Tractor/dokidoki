//! 主动消息生成与投递。

use std::sync::Arc;

use uuid::Uuid;

use crate::{
    chat::{parser, ChatService},
    db::queries::conversations as conversation_queries,
    error::AppError,
};

use super::TriggerFire;

/// 生成并投递；LLM/空气泡失败返回 `Ok(false)`（不计上限）。其它 DB/投递错误上抛。
pub async fn generate_and_deliver(
    chat: &Arc<ChatService>,
    candidate: &conversation_queries::ProactiveCandidateRow,
    fire: &TriggerFire,
) -> Result<bool, AppError> {
    let turn_id = Uuid::new_v4().to_string();

    let request = match crate::chat::build_proactive_request(
        &chat.db,
        &candidate.user_id,
        &candidate.id,
        &turn_id,
        fire,
        chat.summary_config.keep_recent_turns,
    )
    .await
    {
        Ok(request) => request,
        Err(err) => {
            tracing::warn!(
                conversation_id = %candidate.id,
                user_id = %candidate.user_id,
                character_id = %candidate.character_id,
                trigger = fire.as_str(),
                "proactive prompt build failed: {err}"
            );
            return Ok(false);
        }
    };
    tracing::debug!(
        conversation_id = %candidate.id,
        trigger = fire.as_str(),
        message_count = request.messages.len(),
        "proactive prompt assembled"
    );

    let raw = match chat.llm.chat(request).await {
        Ok(raw) => raw,
        Err(err) => {
            tracing::warn!(
                conversation_id = %candidate.id,
                user_id = %candidate.user_id,
                character_id = %candidate.character_id,
                trigger = fire.as_str(),
                "proactive llm failed (skip, not counted): {err}"
            );
            return Ok(false);
        }
    };

    let bubbles = parser::parse_reply(&raw);
    tracing::debug!(
        conversation_id = %candidate.id,
        trigger = fire.as_str(),
        bubbles = bubbles.len(),
        "proactive reply parsed"
    );
    if bubbles.is_empty() {
        tracing::warn!(
            conversation_id = %candidate.id,
            user_id = %candidate.user_id,
            character_id = %candidate.character_id,
            trigger = fire.as_str(),
            raw_len = raw.len(),
            "proactive empty reply (skip, not counted)"
        );
        return Ok(false);
    }

    tracing::debug!(
        conversation_id = %candidate.id,
        trigger = fire.as_str(),
        bubbles = bubbles.len(),
        "proactive delivering bubbles"
    );

    chat.deliver_proactive_bubbles(&candidate.user_id, &candidate.id, &turn_id, bubbles)
        .await?;
    Ok(true)
}
