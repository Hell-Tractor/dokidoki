//! 主动消息调度（M-07）：tick + 闸门 + 触发求值 + 投递。

mod char_schedule;
mod daily_greeting;
mod deliver;
mod gates;
mod pre_sleep;
mod re_engage;
mod schedule_change;
mod silence_wake;
mod triggers;

use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use crate::{
    chat::ChatService,
    db::queries::{
        character_states, characters, conversations as conversation_queries, proactive_logs,
    },
    error::AppError,
};

pub use gates::{is_at_daily_cap, is_blocked_by_dnd, passes_probability_gate};
pub use triggers::{ReEngageReason, TriggerFire};

/// 与 schedule refresh 同循环调用：超时落地 winding_down，再遍历会话投递主动消息。
pub async fn tick(chat: Arc<ChatService>) -> Result<(), AppError> {
    settle_winding_down_timeouts(&chat).await?;

    let candidates = conversation_queries::list_proactive_candidates(&chat.db).await?;
    tracing::debug!(candidates = candidates.len(), "proactive tick begin");

    let now = Utc::now();
    let mut delivered_count = 0u32;

    for candidate in candidates {
        match process_candidate(&chat, &candidate, now).await {
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

async fn settle_winding_down_timeouts(chat: &Arc<ChatService>) -> Result<(), AppError> {
    let timeout = chrono::Duration::seconds(chat.chat_config.winding_down_timeout_secs as i64);
    let older_than = Utc::now() - timeout;
    let rows = conversation_queries::list_winding_down_timed_out(&chat.db, older_than).await?;
    for (id, reason) in rows {
        let reason = reason.unwrap_or(crate::domain::WindingReason::Normal);
        let terminal = reason.terminal_status();
        conversation_queries::enter_terminal_pause(&chat.db, &id, terminal).await?;
        tracing::info!(
            conversation_id = %id,
            status = %terminal,
            winding_reason = %reason,
            "winding_down timed out; entered terminal pause"
        );
    }
    Ok(())
}

/// 返回是否成功投递一条主动消息。
async fn process_candidate(
    chat: &Arc<ChatService>,
    candidate: &conversation_queries::ProactiveCandidateRow,
    now: chrono::DateTime<Utc>,
) -> Result<bool, AppError> {
    let conversation_id = candidate.id.as_str();
    let user_id = candidate.user_id.as_str();
    let character_id = candidate.character_id.as_str();

    if candidate.status == crate::domain::ConversationStatus::WindingDown {
        tracing::trace!(conversation_id, "proactive skipped: winding_down");
        return Ok(false);
    }

    // 同 tick 只加载/解析一次角色日程，供 sleep 闸门与各触发求值复用。
    let schedule =
        char_schedule::load_schedule(&chat.db, character_id, conversation_id).await?;

    if char_schedule::is_in_sleep_slot(schedule.as_ref(), now)? {
        tracing::debug!(
            conversation_id,
            character_id,
            "proactive skipped: kind=sleep slot"
        );
        return Ok(false);
    }

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

    // 按优先级短路：pre_sleep → daily_greeting → re_engage → silence_wake → schedule_change
    let schedule_ref = schedule.as_ref();
    let fire = if let Some(f) =
        pre_sleep::evaluate(chat, candidate, now, schedule_ref).await?
    {
        Some(f)
    } else if let Some(f) =
        daily_greeting::evaluate(chat, candidate, now, schedule_ref).await?
    {
        Some(f)
    } else {
        let availability = candidate
            .availability
            .unwrap_or(crate::domain::Availability::Medium);
        let persona = characters::find_persona(&chat.db, character_id)
            .await?
            .ok_or_else(|| AppError::not_found("角色不存在"))?;
        let base = chat.proactive_config.base_probability(availability);

        if let Some(f) = re_engage::evaluate(candidate, now, &persona.proactive, base) {
            Some(f)
        } else if let Some(f) =
            silence_wake::evaluate(candidate, now, availability, &persona.proactive, base)
        {
            Some(f)
        } else {
            schedule_change::evaluate(
                chat,
                candidate,
                now,
                schedule_ref,
                availability,
                persona.proactive.schedule_change_probability,
                persona.proactive.probability_factor,
            )
            .await?
        }
    };

    let Some(fire) = fire else {
        tracing::trace!(
            conversation_id,
            status = %candidate.status,
            "proactive skipped: no trigger matched"
        );
        return Ok(false);
    };

    tracing::info!(
        conversation_id,
        user_id,
        character_id,
        trigger = fire.as_str(),
        "proactive attempting deliver"
    );

    let delivered = deliver::generate_and_deliver(chat, candidate, &fire).await?;
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
        fire.as_str(),
    )
    .await?;
    character_states::touch_last_proactive_at(&chat.db, character_id, Utc::now()).await?;

    if matches!(fire, TriggerFire::ReEngage { .. }) {
        conversation_queries::enter_active(&chat.db, conversation_id).await?;
        tracing::info!(
            conversation_id,
            "proactive re_engage: conversation status set to active"
        );
    }

    let is_user_birthday = matches!(
        &fire,
        TriggerFire::DailyGreeting {
            special_date_detail: Some(d)
        } if d.contains("生日")
    );
    tracing::info!(
        conversation_id,
        user_id,
        character_id,
        trigger = fire.as_str(),
        is_user_birthday,
        "proactive delivered"
    );

    Ok(true)
}
