//! 主动消息调度（M-07）：tick + 闸门 + 触发求值 + 投递。

mod gates;
mod triggers;

use std::sync::Arc;

use chrono::{TimeZone, Utc};
use uuid::Uuid;

use crate::{
    chat::{parser, ChatService},
    db::queries::{
        character_states, characters, conversations as conversation_queries, proactive_logs, users,
    },
    error::AppError,
    schedule::{self, current_wakeup_slot, in_daily_greeting_window},
    time::parse_timezone,
    utils::{OsUnitRng, UnitRng},
};

pub use gates::{is_at_daily_cap, is_blocked_by_dnd, passes_probability_gate};
pub use triggers::{DailyGreetingExtras, TriggerContext, TriggerType};

/// 与 schedule refresh 同循环调用：超时落地 winding_down，再遍历会话投递主动消息。
pub async fn tick(chat: Arc<ChatService>) -> Result<(), AppError> {
    settle_winding_down_timeouts(&chat).await?;

    let candidates = conversation_queries::list_proactive_candidates(&chat.db).await?;
    tracing::debug!(candidates = candidates.len(), "proactive tick begin");

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
    rng: &mut impl UnitRng,
) -> Result<bool, AppError> {
    let conversation_id = candidate.id.as_str();
    let user_id = candidate.user_id.as_str();
    let character_id = candidate.character_id.as_str();

    if candidate.status == crate::domain::ConversationStatus::WindingDown {
        tracing::trace!(conversation_id, "proactive skipped: winding_down");
        return Ok(false);
    }

    if is_in_sleep_slot(chat, character_id, now).await? {
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

    let availability = candidate.availability.unwrap_or(crate::domain::Availability::Medium);
    let (daily_greeting_eligible, greeting_extras) =
        evaluate_daily_greeting(chat, candidate, now).await?;

    let persona = characters::find_persona(&chat.db, character_id)
        .await?
        .ok_or_else(|| AppError::not_found("角色不存在"))?;

    let (re_engage_eligible, user_busy_curve_p) =
        evaluate_re_engage(candidate, now, &persona.proactive.user_busy_reengage);
    let silence_wake_eligible = evaluate_silence_wake(
        candidate,
        now,
        availability,
        persona.proactive.silence_after_hours,
    );

    let trigger = triggers::select_trigger(&TriggerContext {
        conversation_id,
        status: candidate.status,
        availability,
        daily_greeting_eligible,
        re_engage_eligible,
        silence_wake_eligible,
    });
    let Some(trigger) = trigger else {
        tracing::trace!(
            conversation_id,
            status = %candidate.status,
            availability = %availability,
            daily_greeting_eligible,
            re_engage_eligible,
            silence_wake_eligible,
            "proactive skipped: no trigger matched"
        );
        return Ok(false);
    };

    let probability_factor = persona.proactive.probability_factor;
    let base = chat.proactive_config.base_probability(availability);
    let pass_probability = match (trigger, user_busy_curve_p) {
        (TriggerType::ReEngage, Some(curve_p)) => {
            // paused_user_busy：P(t) × 全局/availability 闸门
            let final_p = (curve_p * base * probability_factor).clamp(0.0, 1.0);
            let roll = rng.next_unit();
            let ok = roll < final_p;
            if !ok {
                tracing::debug!(
                    conversation_id,
                    curve_p,
                    base,
                    probability_factor,
                    final_p,
                    roll,
                    "proactive skipped: user_busy re_engage curve"
                );
            }
            ok
        }
        _ => {
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
                    availability = %availability,
                    probability_factor,
                    base,
                    "proactive skipped: probability gate"
                );
                false
            } else {
                true
            }
        }
    };
    if !pass_probability {
        return Ok(false);
    }

    tracing::info!(
        conversation_id,
        user_id,
        character_id,
        trigger = trigger.as_str(),
        availability = %availability,
        "proactive attempting deliver"
    );

    let delivered =
        generate_and_deliver(chat, candidate, trigger, &greeting_extras).await?;
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
        conversation_queries::enter_active(&chat.db, conversation_id).await?;
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
        is_user_birthday = greeting_extras.is_user_birthday,
        "proactive delivered"
    );

    Ok(true)
}

async fn is_in_sleep_slot(
    chat: &Arc<ChatService>,
    character_id: &str,
    now: chrono::DateTime<Utc>,
) -> Result<bool, AppError> {
    let Some(raw) = characters::find_schedule_json(&chat.db, character_id).await? else {
        return Ok(false);
    };
    let Some(schedule) = schedule::Schedule::try_from_json_value(raw)? else {
        return Ok(false);
    };
    Ok(schedule::current_slot_kind(&schedule, now)? == Some(schedule::SlotKind::Sleep))
}

async fn evaluate_daily_greeting(
    chat: &Arc<ChatService>,
    candidate: &conversation_queries::ProactiveCandidateRow,
    now: chrono::DateTime<Utc>,
) -> Result<(bool, DailyGreetingExtras), AppError> {
    let mut extras = DailyGreetingExtras::default();

    let user = users::find_by_id(&chat.db, &candidate.user_id)
        .await?
        .ok_or_else(|| AppError::not_found("用户不存在"))?;
    extras.user_birthday = user.birthday;
    extras.is_user_birthday =
        triggers::is_birthday_today(user.birthday, now, &candidate.timezone)?;

    let Some(raw) = characters::find_schedule_json(&chat.db, &candidate.character_id).await?
    else {
        tracing::trace!(
            conversation_id = %candidate.id,
            character_id = %candidate.character_id,
            "daily_greeting: character has no schedule_json"
        );
        return Ok((false, extras));
    };
    let schedule = match schedule::Schedule::try_from_json_value(raw) {
        Ok(None) => {
            tracing::trace!(
                conversation_id = %candidate.id,
                character_id = %candidate.character_id,
                "daily_greeting: empty schedule_json"
            );
            return Ok((false, extras));
        }
        Ok(Some(schedule)) => schedule,
        Err(err) => {
            tracing::warn!(
                conversation_id = %candidate.id,
                character_id = %candidate.character_id,
                "daily_greeting: invalid schedule_json: {err}"
            );
            return Ok((false, extras));
        }
    };

    let Some(wakeup) = current_wakeup_slot(&schedule, now)? else {
        tracing::trace!(
            conversation_id = %candidate.id,
            character_id = %candidate.character_id,
            "daily_greeting: not in kind=wake slot"
        );
        return Ok((false, extras));
    };

    let cfg = &chat.proactive_config;
    if !in_daily_greeting_window(
        &wakeup,
        &candidate.character_id,
        cfg.daily_greeting_window_min_mins,
        cfg.daily_greeting_window_max_mins,
    ) {
        tracing::trace!(
            conversation_id = %candidate.id,
            activity = %wakeup.activity,
            minutes_into_slot = wakeup.minutes_into_slot,
            "daily_greeting: outside greeting window"
        );
        return Ok((false, extras));
    }

    // 「每角色每日一次」按角色日程时区自然日统计。
    let (char_day_start, char_day_end) =
        character_local_day_bounds(now, &schedule.timezone, wakeup.local_date)?;
    let already = proactive_logs::count_trigger_between(
        &chat.db,
        &candidate.user_id,
        &candidate.character_id,
        TriggerType::DailyGreeting.as_str(),
        char_day_start,
        char_day_end,
    )
    .await?;
    if already > 0 {
        tracing::trace!(
            conversation_id = %candidate.id,
            character_id = %candidate.character_id,
            "daily_greeting: already sent today"
        );
        return Ok((false, extras));
    }

    tracing::debug!(
        conversation_id = %candidate.id,
        activity = %wakeup.activity,
        minutes_into_slot = wakeup.minutes_into_slot,
        is_user_birthday = extras.is_user_birthday,
        "daily_greeting eligible"
    );
    Ok((true, extras))
}

fn evaluate_re_engage(
    candidate: &conversation_queries::ProactiveCandidateRow,
    now: chrono::DateTime<Utc>,
    user_busy_curve: &crate::domain::persona::UserBusyReengage,
) -> (bool, Option<f64>) {
    if triggers::is_char_busy_re_engage_ready(
        candidate.status,
        candidate.activity_ends_at,
        now,
    ) {
        tracing::debug!(
            conversation_id = %candidate.id,
            activity_ends_at = ?candidate.activity_ends_at,
            "re_engage eligible: paused_char_busy after activity end"
        );
        return (true, None);
    }

    if let Some(curve_p) = triggers::user_busy_re_engage_probability(
        candidate.status,
        candidate.paused_at,
        now,
        user_busy_curve,
    ) {
        if curve_p > 0.0 {
            tracing::debug!(
                conversation_id = %candidate.id,
                curve_p,
                "re_engage eligible: paused_user_busy curve"
            );
            return (true, Some(curve_p));
        }
        tracing::trace!(
            conversation_id = %candidate.id,
            "re_engage: paused_user_busy still in min_delay"
        );
    }
    (false, None)
}

fn evaluate_silence_wake(
    candidate: &conversation_queries::ProactiveCandidateRow,
    now: chrono::DateTime<Utc>,
    availability: crate::domain::Availability,
    silence_after_hours: f64,
) -> bool {
    let eligible = triggers::is_silence_wake_eligible(
        candidate.status,
        candidate.last_user_message_at,
        now,
        silence_after_hours,
        availability,
    );
    if eligible {
        tracing::debug!(
            conversation_id = %candidate.id,
            last_user_message_at = ?candidate.last_user_message_at,
            silence_after_hours,
            availability = %availability,
            "silence_wake eligible"
        );
    }
    eligible
}

fn character_local_day_bounds(
    now: chrono::DateTime<Utc>,
    tz: &str,
    local_date: chrono::NaiveDate,
) -> Result<(chrono::DateTime<Utc>, chrono::DateTime<Utc>), AppError> {
    let _ = now;
    let tz = parse_timezone(tz)?;
    let start = local_date
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| AppError::bad_request("无效的本地日界线"))?;
    let end = start + chrono::Duration::days(1);
    Ok((
        tz.from_local_datetime(&start)
            .single()
            .ok_or_else(|| AppError::bad_request("无效的本地日界线"))?
            .with_timezone(&Utc),
        tz.from_local_datetime(&end)
            .single()
            .ok_or_else(|| AppError::bad_request("无效的本地日界线"))?
            .with_timezone(&Utc),
    ))
}

fn special_date_detail(extras: &DailyGreetingExtras) -> Option<String> {
    if !extras.has_special_date() {
        return None;
    }
    let mut parts = Vec::new();
    if extras.is_user_birthday {
        if let Some(date) = extras.user_birthday {
            parts.push(format!("对方生日（{}）", date.format("%m-%d")));
        } else {
            parts.push("对方生日".into());
        }
    }
    for name in &extras.holiday_names {
        parts.push(name.clone());
    }
    Some(parts.join("、"))
}

/// 生成并投递；LLM/空气泡失败返回 `Ok(false)`（不计上限）。其它 DB/投递错误上抛。
async fn generate_and_deliver(
    chat: &Arc<ChatService>,
    candidate: &conversation_queries::ProactiveCandidateRow,
    trigger: TriggerType,
    greeting_extras: &DailyGreetingExtras,
) -> Result<bool, AppError> {
    let turn_id = Uuid::new_v4().to_string();
    let special = if trigger == TriggerType::DailyGreeting {
        special_date_detail(greeting_extras)
    } else {
        None
    };

    let request = match crate::chat::build_proactive_request(
        &chat.db,
        &candidate.user_id,
        &candidate.id,
        &turn_id,
        trigger.as_str(),
        chat.summary_config.keep_recent_turns,
        special.as_deref(),
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
