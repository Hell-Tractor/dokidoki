use sqlx::MySqlPool;

use crate::{
    db::{models::Conversation, queries::{characters, conversations, memories, messages, users}},
    domain::persona::Persona,
    error::AppError,
    llm::{ChatRequest, LlmMessage},
    prompt::{
        build_icebreaker_system_prompt, build_system_prompt, format_icebreaker_user_message,
        CurrentStatePrompt,
    },
    schedule,
};

struct PromptContext {
    persona: Persona,
    character_name: String,
    user_display_name: String,
    current_state: Option<CurrentStatePrompt>,
    memories: Vec<String>,
    summary: Option<String>,
}

pub async fn build_chat_request(
    pool: &MySqlPool,
    user_id: &str,
    conversation_id: &str,
    turn_id: &str,
    keep_recent_turns: u32,
) -> Result<ChatRequest, AppError> {
    let conversation = load_owned_conversation(pool, user_id, conversation_id).await?;
    let ctx = load_prompt_context(pool, user_id, &conversation).await?;

    let system = build_system_prompt(
        &ctx.persona,
        &ctx.character_name,
        &ctx.user_display_name,
        ctx.current_state.as_ref(),
        &ctx.memories,
        ctx.summary.as_deref(),
    );

    let recent =
        messages::list_text_messages_for_recent_turns(pool, conversation_id, keep_recent_turns)
            .await?;

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
        &ctx.persona,
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

/// 主动消息 Prompt（骨架：T-01～T-05 + 临时场景说明；正式 T-12～T-18 后续落地）。
pub async fn build_proactive_request(
    pool: &MySqlPool,
    user_id: &str,
    conversation_id: &str,
    turn_id: &str,
    trigger: &str,
    keep_recent_turns: u32,
) -> Result<ChatRequest, AppError> {
    let conversation = load_owned_conversation(pool, user_id, conversation_id).await?;
    let ctx = load_prompt_context(pool, user_id, &conversation).await?;

    let mut system = build_system_prompt(
        &ctx.persona,
        &ctx.character_name,
        &ctx.user_display_name,
        ctx.current_state.as_ref(),
        &ctx.memories,
        ctx.summary.as_deref(),
    );
    system.push_str("\n\n【主动场景：占位】\n");
    system.push_str("你正在主动找对方说话。语气符合人设与当前状态。\n");
    system.push_str("输出格式用 [REPLY]，1～3 条短气泡。不要提系统或任务。\n");

    let recent =
        messages::list_text_messages_for_recent_turns(pool, conversation_id, keep_recent_turns)
            .await?;

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

    llm_messages.push(LlmMessage {
        role: "user".into(),
        content: format!(
            "【系统任务：主动发起消息】\n触发类型：{trigger}\n请由你主动给对方发消息，不要等用户先说话。"
        ),
    });

    Ok(ChatRequest {
        conversation_id: conversation_id.to_owned(),
        turn_id: turn_id.to_owned(),
        messages: llm_messages,
    })
}

async fn load_owned_conversation(
    pool: &MySqlPool,
    user_id: &str,
    conversation_id: &str,
) -> Result<Conversation, AppError> {
    let conversation = conversations::find_by_id(pool, conversation_id)
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

    let persona = characters::find_persona(pool, &conversation.character_id)
        .await?
        .ok_or_else(|| AppError::not_found("角色不存在"))?;

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

    let memory_rows = memories::list_active(pool, user_id, &conversation.character_id).await?;
    let memories = memory_rows.into_iter().map(|row| row.content).collect();

    let summary = conversations::find_summary_fields(pool, &conversation.id)
        .await?
        .and_then(|fields| fields.summary);

    Ok(PromptContext {
        persona,
        character_name: character.name,
        user_display_name: user.display_name,
        current_state,
        memories,
        summary,
    })
}
