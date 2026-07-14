//! 主动消息调度（M-07）：tick + 闸门 + 触发求值 + 投递。
//!
//! 本阶段完成骨架与闸门；各类触发器仍为 stub（不命中），故生产 tick 不会发消息。

mod gates;
mod triggers;

use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use crate::{
    chat::{parser, ChatService},
    db::queries::{
        character_states, characters, conversations as conversation_queries, proactive_logs,
    },
    error::AppError,
    utils::{OsUnitRng, UnitRng},
};

pub use gates::{is_at_daily_cap, is_blocked_by_dnd, passes_probability_gate};
pub use triggers::{TriggerContext, TriggerType};

/// 与 schedule refresh 同循环调用：遍历会话对，闸门通过且触发命中则生成并投递。
pub async fn tick(chat: Arc<ChatService>) -> Result<(), AppError> {
    let candidates = conversation_queries::list_proactive_candidates(&chat.db).await?;
    let mut rng = OsUnitRng;
    let now = Utc::now();

    for candidate in candidates {
        if let Err(err) = process_candidate(&chat, &candidate, now, &mut rng).await {
            tracing::warn!(
                conversation_id = %candidate.id,
                user_id = %candidate.user_id,
                "proactive candidate failed: {err}"
            );
        }
    }

    Ok(())
}

async fn process_candidate(
    chat: &Arc<ChatService>,
    candidate: &conversation_queries::ProactiveCandidateRow,
    now: chrono::DateTime<Utc>,
    rng: &mut impl UnitRng,
) -> Result<(), AppError> {
    if is_blocked_by_dnd(
        now,
        &candidate.timezone,
        candidate.dnd_start,
        candidate.dnd_end,
    )? {
        return Ok(());
    }

    let (day_start, day_end) = gates::day_bounds_for_user(now, &candidate.timezone)?;
    let sent_today =
        proactive_logs::count_for_user_between(&chat.db, &candidate.user_id, day_start, day_end)
            .await?;
    if is_at_daily_cap(sent_today, candidate.max_proactive_per_day) {
        return Ok(());
    }

    if chat.has_active_delivery(&candidate.id).await {
        return Ok(());
    }

    let availability = candidate
        .availability
        .as_deref()
        .unwrap_or("medium");

    let trigger = triggers::select_trigger(&TriggerContext {
        conversation_id: &candidate.id,
        status: &candidate.status,
        availability,
    });
    let Some(trigger) = trigger else {
        return Ok(());
    };

    let persona = characters::find_persona(&chat.db, &candidate.character_id)
        .await?
        .ok_or_else(|| AppError::not_found("角色不存在"))?;
    if !passes_probability_gate(
        &chat.proactive_config,
        availability,
        persona.proactive.probability_factor,
        rng,
    ) {
        return Ok(());
    }

    let delivered = generate_and_deliver(chat, candidate, trigger).await?;
    if !delivered {
        return Ok(());
    }

    let log_id = Uuid::new_v4().to_string();
    proactive_logs::insert(
        &chat.db,
        &log_id,
        &candidate.user_id,
        &candidate.character_id,
        &candidate.id,
        trigger.as_str(),
    )
    .await?;
    character_states::touch_last_proactive_at(&chat.db, &candidate.character_id, Utc::now())
        .await?;

    if trigger == TriggerType::ReEngage {
        conversation_queries::update_status(&chat.db, &candidate.id, "active", false).await?;
    }

    Ok(())
}

/// 生成并投递；LLM/空气泡失败返回 `Ok(false)`（不计上限）。其它 DB/投递错误上抛。
async fn generate_and_deliver(
    chat: &Arc<ChatService>,
    candidate: &conversation_queries::ProactiveCandidateRow,
    trigger: TriggerType,
) -> Result<bool, AppError> {
    let turn_id = Uuid::new_v4().to_string();
    let request = crate::chat::build_proactive_request(
        &chat.db,
        &candidate.user_id,
        &candidate.id,
        &turn_id,
        trigger.as_str(),
        chat.summary_config.keep_recent_turns,
    )
    .await?;

    let raw = match chat.llm.chat(request).await {
        Ok(raw) => raw,
        Err(err) => {
            tracing::warn!(
                conversation_id = %candidate.id,
                trigger = trigger.as_str(),
                "proactive llm failed: {err}"
            );
            return Ok(false);
        }
    };

    let bubbles = parser::parse_reply(&raw);
    if bubbles.is_empty() {
        return Ok(false);
    }

    chat.deliver_proactive_bubbles(&candidate.user_id, &candidate.id, &turn_id, bubbles)
        .await?;
    Ok(true)
}
