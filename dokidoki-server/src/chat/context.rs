use serde_json::Value;
use sqlx::MySqlPool;

use crate::{
    db::{models::Conversation, queries::{characters, messages, users}},
    error::AppError,
    llm::{ChatRequest, LlmMessage},
    persona::{
        build_icebreaker_system_prompt, build_system_prompt, format_icebreaker_user_message,
        CurrentStatePrompt,
    },
    schedule,
};

const RECENT_MESSAGE_LIMIT: u32 = 20;

struct PromptContext {
    persona_json: Value,
    character_name: String,
    user_display_name: String,
    current_state: Option<CurrentStatePrompt>,
}

pub async fn build_chat_request(
    pool: &MySqlPool,
    user_id: &str,
    conversation_id: &str,
    turn_id: &str,
) -> Result<ChatRequest, AppError> {
    let conversation = load_owned_conversation(pool, user_id, conversation_id).await?;
    let ctx = load_prompt_context(pool, user_id, &conversation).await?;

    let system = build_system_prompt(
        &ctx.persona_json,
        &ctx.character_name,
        &ctx.user_display_name,
        ctx.current_state.as_ref(),
    );

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
                tracing::warn!(
                    "Unknown message role: {}, skipping message(id = {}, user_id = {}, conversation_id = {})",
                    message.role,
                    message.id,
                    user_id,
                    conversation_id
                );
                continue;
            }
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

pub async fn build_icebreaker_request(
    pool: &MySqlPool,
    user_id: &str,
    conversation_id: &str,
    turn_id: &str,
) -> Result<ChatRequest, AppError> {
    let conversation = load_owned_conversation(pool, user_id, conversation_id).await?;
    let ctx = load_prompt_context(pool, user_id, &conversation).await?;

    let system = build_icebreaker_system_prompt(
        &ctx.persona_json,
        &ctx.character_name,
        &ctx.user_display_name,
        ctx.current_state.as_ref(),
    );

    Ok(ChatRequest {
        conversation_id: conversation_id.to_owned(),
        turn_id: turn_id.to_owned(),
        messages: vec![
            LlmMessage {
                role: "system".into(),
                content: system,
            },
            LlmMessage {
                role: "user".into(),
                content: format_icebreaker_user_message().into(),
            },
        ],
    })
}

async fn load_owned_conversation(
    pool: &MySqlPool,
    user_id: &str,
    conversation_id: &str,
) -> Result<Conversation, AppError> {
    let conversation = crate::db::queries::conversations::find_by_id(pool, conversation_id)
        .await?
        .ok_or_else(|| AppError::not_found("会话不存在"))?;

    if conversation.user_id != user_id {
        return Err(AppError::not_found("会话不存在"));
    }

    Ok(conversation)
}

async fn load_prompt_context(
    pool: &MySqlPool,
    user_id: &str,
    conversation: &Conversation,
) -> Result<PromptContext, AppError> {
    let user = users::find_by_id(pool, user_id)
        .await?
        .ok_or_else(|| AppError::not_found("用户不存在"))?;

    let character = characters::find_by_id(pool, &conversation.character_id)
        .await?
        .ok_or_else(|| AppError::not_found("角色不存在"))?;

    let persona_json = characters::find_persona_json(pool, &conversation.character_id)
        .await?
        .unwrap_or_else(|| serde_json::json!({}));

    let current_state = schedule::load_current_state_for_prompt(pool, &conversation.character_id)
        .await
        .inspect_err(|err| tracing::warn!("schedule state unavailable: {err}"))
        .ok()
        .flatten()
        .map(|state| CurrentStatePrompt {
            weekday_zh: state.weekday_zh,
            time_hm: state.time_hm,
            activity: state.activity,
            mood: state.mood,
            availability: state.availability,
            random_event: state.random_event,
        });

    Ok(PromptContext {
        persona_json,
        character_name: character.name,
        user_display_name: user.display_name,
        current_state,
    })
}
