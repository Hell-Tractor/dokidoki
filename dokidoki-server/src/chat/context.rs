use sqlx::MySqlPool;

use crate::{
    db::queries::{characters, conversations, messages, users},
    error::AppError,
    llm::{ChatRequest, LlmMessage},
    persona::build_system_prompt,
};

const RECENT_MESSAGE_LIMIT: u32 = 20;

pub async fn build_chat_request(
    pool: &MySqlPool,
    user_id: &str,
    conversation_id: &str,
    turn_id: &str,
) -> Result<ChatRequest, AppError> {
    let conversation = conversations::find_by_id(pool, conversation_id)
        .await?
        .ok_or_else(|| AppError::not_found("会话不存在"))?;

    if conversation.user_id != user_id {
        return Err(AppError::not_found("会话不存在"));
    }

    let user = users::find_by_id(pool, user_id)
        .await?
        .ok_or_else(|| AppError::not_found("用户不存在"))?;

    let character = characters::find_by_id(pool, &conversation.character_id)
        .await?
        .ok_or_else(|| AppError::not_found("角色不存在"))?;

    let persona_json = characters::find_persona_json(pool, &conversation.character_id)
        .await?
        .unwrap_or_else(|| serde_json::json!({}));

    let system = build_system_prompt(&persona_json, &character.name, &user.display_name);

    let recent = messages::list_recent_text(pool, conversation_id, RECENT_MESSAGE_LIMIT).await?;

    let mut llm_messages = vec![LlmMessage {
        role: "system".into(),
        content: system,
    }];

    for message in recent {
        let role = match message.role.as_str() {
            "user" => "user",
            "character" => "assistant",
            _ => {
                tracing::warn!("Unknown message role: {}, skipping message(id = {}, user_id = {}, conversation_id = {})", message.role, message.id, user_id, conversation_id);
                continue;
            },
        };
        let content = message.content.unwrap_or_default();
        if content.is_empty() {
            continue;
        }
        llm_messages.push(LlmMessage {
            role: role.into(),
            content,
        });
    }

    Ok(ChatRequest {
        conversation_id: conversation_id.to_owned(),
        turn_id: turn_id.to_owned(),
        messages: llm_messages,
    })
}
