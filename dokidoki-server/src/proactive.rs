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
    tracing::debug!(
        candidates = candidates.len(),
        "proactive tick begin"
    );

    let mut rng = OsUnitRng;
    let now = Utc::now();
    let mut delivered_count = 0u32;

    for candidate in candidates {
        match process_candidate(&chat, &candidate, now, &mut rng).await {
            Ok(true) => delivered_count += 1,
            Ok(false) => {}
            Err(err) => {
                tracing::warn!(
                    conversation_id = %candidate.id,
                    user_id = %candidate.user_id,
                    character_id = %candidate.character_id,
                    "proactive candidate failed: {err}"
                );
            }
        }
    }

    if delivered_count > 0 {
        tracing::info!(delivered = delivered_count, "proactive tick finished");
    } else {
        tracing::debug!(delivered = 0, "proactive tick finished");
    }

    Ok(())
}

/// 返回是否成功投递一条主动消息。
async fn process_candidate(
    chat: &Arc<ChatService>,
    candidate: &conversation_queries::ProactiveCandidateRow,
    now: chrono::DateTime<Utc>,
    rng: &mut impl UnitRng,
) -> Result<bool, AppError> {
    let conversation_id = candidate.id.as_str();
    let user_id = candidate.user_id.as_str();
    let character_id = candidate.character_id.as_str();

    if is_blocked_by_dnd(
        now,
        &candidate.timezone,
        candidate.dnd_start,
        candidate.dnd_end,
    )? {
        tracing::debug!(
            conversation_id,
            user_id,
            character_id,
            timezone = %candidate.timezone,
            "proactive skipped: dnd window"
        );
        return Ok(false);
    }

    let (day_start, day_end) = gates::day_bounds_for_user(now, &candidate.timezone)?;
    let sent_today =
        proactive_logs::count_for_user_between(&chat.db, user_id, day_start, day_end).await?;
    if is_at_daily_cap(sent_today, candidate.max_proactive_per_day) {
        tracing::debug!(
            conversation_id,
            user_id,
            sent_today,
            max = candidate.max_proactive_per_day,
            "proactive skipped: daily cap"
        );
        return Ok(false);
    }

    if chat.has_active_delivery(conversation_id).await {
        tracing::debug!(
            conversation_id,
            user_id,
            character_id,
            "proactive skipped: active delivery in progress"
        );
        return Ok(false);
    }

    let availability = candidate.availability.as_deref().unwrap_or("medium");

    let trigger = triggers::select_trigger(&TriggerContext {
        conversation_id,
        status: &candidate.status,
        availability,
    });
    let Some(trigger) = trigger else {
        tracing::trace!(
            conversation_id,
            status = %candidate.status,
            availability,
            "proactive skipped: no trigger matched"
        );
        return Ok(false);
    };

    let persona = characters::find_persona(&chat.db, character_id)
        .await?
        .ok_or_else(|| AppError::not_found("角色不存在"))?;
    let probability_factor = persona.proactive.probability_factor;
    if !passes_probability_gate(
        &chat.proactive_config,
        availability,
        probability_factor,
        rng,
    ) {
        tracing::debug!(
            conversation_id,
            user_id,
            character_id,
            trigger = trigger.as_str(),
            availability,
            probability_factor,
            base = chat.proactive_config.base_probability(availability),
            "proactive skipped: probability gate"
        );
        return Ok(false);
    }

    tracing::info!(
        conversation_id,
        user_id,
        character_id,
        trigger = trigger.as_str(),
        availability,
        "proactive attempting deliver"
    );

    let delivered = generate_and_deliver(chat, candidate, trigger).await?;
    if !delivered {
        return Ok(false);
    }

    let log_id = Uuid::new_v4().to_string();
    proactive_logs::insert(
        &chat.db,
        &log_id,
        user_id,
        character_id,
        conversation_id,
        trigger.as_str(),
    )
    .await?;
    character_states::touch_last_proactive_at(&chat.db, character_id, Utc::now()).await?;

    if trigger == TriggerType::ReEngage {
        conversation_queries::update_status(&chat.db, conversation_id, "active", false).await?;
        tracing::info!(
            conversation_id,
            "proactive re_engage: conversation status set to active"
        );
    }

    tracing::info!(
        conversation_id,
        user_id,
        character_id,
        trigger = trigger.as_str(),
        "proactive delivered"
    );

    Ok(true)
}

/// 生成并投递；LLM/空气泡失败返回 `Ok(false)`（不计上限）。其它 DB/投递错误上抛。
async fn generate_and_deliver(
    chat: &Arc<ChatService>,
    candidate: &conversation_queries::ProactiveCandidateRow,
    trigger: TriggerType,
) -> Result<bool, AppError> {
    let turn_id = Uuid::new_v4().to_string();
    let request = match crate::chat::build_proactive_request(
        &chat.db,
        &candidate.user_id,
        &candidate.id,
        &turn_id,
        trigger.as_str(),
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
                trigger = trigger.as_str(),
                "proactive prompt build failed: {err}"
            );
            return Ok(false);
        }
    };

    let raw = match chat.llm.chat(request).await {
        Ok(raw) => raw,
        Err(err) => {
            tracing::warn!(
                conversation_id = %candidate.id,
                user_id = %candidate.user_id,
                character_id = %candidate.character_id,
                trigger = trigger.as_str(),
                "proactive llm failed (skip, not counted): {err}"
            );
            return Ok(false);
        }
    };

    let bubbles = parser::parse_reply(&raw);
    if bubbles.is_empty() {
        tracing::warn!(
            conversation_id = %candidate.id,
            user_id = %candidate.user_id,
            character_id = %candidate.character_id,
            trigger = trigger.as_str(),
            raw_len = raw.len(),
            "proactive empty reply (skip, not counted)"
        );
        return Ok(false);
    }

    tracing::debug!(
        conversation_id = %candidate.id,
        trigger = trigger.as_str(),
        bubbles = bubbles.len(),
        "proactive delivering bubbles"
    );

    chat.deliver_proactive_bubbles(&candidate.user_id, &candidate.id, &turn_id, bubbles)
        .await?;
    Ok(true)
}
