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
    schedule::{
        self, at_daily_greeting_fire_time, at_pre_sleep_fire_time, current_custom_slot,
        current_wakeup_slot, in_schedule_change_window, upcoming_pre_sleep,
    },
    time::parse_timezone,
};

pub use gates::{is_at_daily_cap, is_blocked_by_dnd, passes_probability_gate};
pub use triggers::{
    DailyGreetingExtras, PreSleepExtras, ReEngageExtras, ReEngageReason, ScheduleChangeExtras,
    TriggerContext, TriggerType,
};

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
    let (pre_sleep_eligible, pre_sleep_extras) =
        evaluate_pre_sleep(chat, candidate, now).await?;
    let (daily_greeting_eligible, greeting_extras) =
        evaluate_daily_greeting(chat, candidate, now).await?;

    let persona = characters::find_persona(&chat.db, character_id)
        .await?
        .ok_or_else(|| AppError::not_found("角色不存在"))?;

    let (re_engage_eligible, re_engage_extras) = evaluate_re_engage(
        candidate,
        now,
        &persona.proactive,
        chat.proactive_config.base_probability(availability),
    );
    let silence_wake_eligible = evaluate_silence_wake(
        candidate,
        now,
        availability,
        &persona.proactive,
        chat.proactive_config.base_probability(availability),
    );
    let (schedule_change_eligible, schedule_change_extras) = evaluate_schedule_change(
        chat,
        candidate,
        now,
        availability,
        persona.proactive.schedule_change_probability,
        persona.proactive.probability_factor,
    )
    .await?;

    let trigger = triggers::select_trigger(&TriggerContext {
        conversation_id,
        status: candidate.status,
        availability,
        pre_sleep_eligible,
        daily_greeting_eligible,
        re_engage_eligible,
        silence_wake_eligible,
        schedule_change_eligible,
    });
    let Some(trigger) = trigger else {
        tracing::trace!(
            conversation_id,
            status = %candidate.status,
            availability = %availability,
            pre_sleep_eligible,
            daily_greeting_eligible,
            re_engage_eligible,
            silence_wake_eligible,
            schedule_change_eligible,
            "proactive skipped: no trigger matched"
        );
        return Ok(false);
    };

    tracing::info!(
        conversation_id,
        user_id,
        character_id,
        trigger = trigger.as_str(),
        availability = %availability,
        "proactive attempting deliver"
    );

    let delivered = generate_and_deliver(
        chat,
        candidate,
        trigger,
        &greeting_extras,
        &pre_sleep_extras,
        &re_engage_extras,
        &schedule_change_extras,
    )
    .await?;
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

async fn evaluate_pre_sleep(
    chat: &Arc<ChatService>,
    candidate: &conversation_queries::ProactiveCandidateRow,
    now: chrono::DateTime<Utc>,
) -> Result<(bool, PreSleepExtras), AppError> {
    let extras = PreSleepExtras {
        ask_user_busy_care: candidate.status
            == crate::domain::ConversationStatus::PausedCharBusy,
    };

    let Some(raw) = characters::find_schedule_json(&chat.db, &candidate.character_id).await?
    else {
        return Ok((false, extras));
    };
    let schedule = match schedule::Schedule::try_from_json_value(raw) {
        Ok(None) => return Ok((false, extras)),
        Ok(Some(schedule)) => schedule,
        Err(err) => {
            tracing::warn!(
                conversation_id = %candidate.id,
                character_id = %candidate.character_id,
                "pre_sleep: invalid schedule_json: {err}"
            );
            return Ok((false, extras));
        }
    };

    let Some(status) = upcoming_pre_sleep(&schedule, now)? else {
        tracing::trace!(
            conversation_id = %candidate.id,
            character_id = %candidate.character_id,
            "pre_sleep: no upcoming sleep window"
        );
        return Ok((false, extras));
    };

    let cfg = &chat.proactive_config;
    if !at_pre_sleep_fire_time(
        &status,
        &candidate.character_id,
        cfg.pre_sleep_window_min_mins,
        cfg.pre_sleep_window_max_mins,
    ) {
        tracing::trace!(
            conversation_id = %candidate.id,
            minutes_until_sleep = status.minutes_until_sleep,
            "pre_sleep: before fixed fire time"
        );
        return Ok((false, extras));
    }

    let (char_day_start, char_day_end) =
        character_local_day_bounds(now, &schedule.timezone, status.sleep_local_date)?;
    let already = proactive_logs::count_trigger_between(
        &chat.db,
        &candidate.user_id,
        &candidate.character_id,
        TriggerType::PreSleep.as_str(),
        char_day_start,
        char_day_end,
    )
    .await?;
    if already > 0 {
        tracing::trace!(
            conversation_id = %candidate.id,
            character_id = %candidate.character_id,
            "pre_sleep: already sent today"
        );
        return Ok((false, extras));
    }

    tracing::debug!(
        conversation_id = %candidate.id,
        minutes_until_sleep = status.minutes_until_sleep,
        ask_user_busy_care = extras.ask_user_busy_care,
        "pre_sleep eligible"
    );
    Ok((true, extras))
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
    if !at_daily_greeting_fire_time(
        &wakeup,
        &candidate.character_id,
        cfg.daily_greeting_window_min_mins,
        cfg.daily_greeting_window_max_mins,
    ) {
        tracing::trace!(
            conversation_id = %candidate.id,
            activity = %wakeup.activity,
            minutes_into_slot = wakeup.minutes_into_slot,
            "daily_greeting: before fixed fire time"
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
    proactive: &crate::domain::persona::ProactiveConfig,
    availability_base: f64,
) -> (bool, ReEngageExtras) {
    let mut extras = ReEngageExtras::default();
    let character_id = candidate.character_id.as_str();
    let factor = proactive.probability_factor;

    if triggers::is_char_busy_re_engage_ready(
        candidate.status,
        candidate.activity_ends_at,
        now,
    ) {
        let Some(anchor) = candidate.activity_ends_at else {
            return (false, extras);
        };
        let interval = triggers::retry_interval_mins(
            character_id,
            anchor,
            proactive.re_engage_retry_min_minutes,
            proactive.re_engage_retry_max_minutes,
            "re_engage_char",
        );
        let Some(bucket) = triggers::retry_bucket_index(now, anchor, interval) else {
            return (false, extras);
        };
        let final_p = (availability_base * factor).clamp(0.0, 1.0);
        let (pass, roll) = triggers::retry_bucket_probability_passes(
            character_id,
            anchor,
            bucket,
            "re_engage_char",
            final_p,
        );
        if !pass {
            tracing::debug!(
                conversation_id = %candidate.id,
                bucket,
                interval,
                final_p,
                roll,
                "re_engage: char_busy bucket miss"
            );
            return (false, extras);
        }
        extras.reason = Some(ReEngageReason::CharBusy);
        tracing::debug!(
            conversation_id = %candidate.id,
            activity_ends_at = ?candidate.activity_ends_at,
            bucket,
            interval,
            final_p,
            "re_engage eligible: paused_char_busy"
        );
        return (true, extras);
    }

    if let Some(curve_p) = triggers::user_busy_re_engage_probability(
        candidate.status,
        candidate.paused_at,
        now,
        &proactive.user_busy_reengage,
    ) {
        if curve_p <= 0.0 {
            tracing::trace!(
                conversation_id = %candidate.id,
                "re_engage: paused_user_busy still in min_delay"
            );
            return (false, extras);
        }
        let Some(anchor) = candidate.paused_at else {
            return (false, extras);
        };
        let curve_start = anchor
            + chrono::Duration::minutes(i64::from(proactive.user_busy_reengage.min_delay_minutes));
        let interval = triggers::retry_interval_mins(
            character_id,
            curve_start,
            proactive.re_engage_retry_min_minutes,
            proactive.re_engage_retry_max_minutes,
            "re_engage_user",
        );
        let Some(bucket) = triggers::retry_bucket_index(now, curve_start, interval) else {
            return (false, extras);
        };
        let final_p = (curve_p * availability_base * factor).clamp(0.0, 1.0);
        let (pass, roll) = triggers::retry_bucket_probability_passes(
            character_id,
            curve_start,
            bucket,
            "re_engage_user",
            final_p,
        );
        if !pass {
            tracing::debug!(
                conversation_id = %candidate.id,
                bucket,
                interval,
                curve_p,
                final_p,
                roll,
                "re_engage: user_busy bucket miss"
            );
            return (false, extras);
        }
        extras.reason = Some(ReEngageReason::UserBusy);
        tracing::debug!(
            conversation_id = %candidate.id,
            curve_p,
            bucket,
            interval,
            final_p,
            "re_engage eligible: paused_user_busy"
        );
        return (true, extras);
    }
    (false, extras)
}

fn evaluate_silence_wake(
    candidate: &conversation_queries::ProactiveCandidateRow,
    now: chrono::DateTime<Utc>,
    availability: crate::domain::Availability,
    proactive: &crate::domain::persona::ProactiveConfig,
    availability_base: f64,
) -> bool {
    if !triggers::is_silence_wake_eligible(
        candidate.status,
        candidate.last_user_message_at,
        now,
        proactive.silence_after_hours,
        availability,
    ) {
        return false;
    }
    let Some(last_user) = candidate.last_user_message_at else {
        return false;
    };
    let silence_ms = (proactive.silence_after_hours * 3_600_000.0).round() as i64;
    let anchor = last_user + chrono::Duration::milliseconds(silence_ms);
    let interval = triggers::retry_interval_mins(
        &candidate.character_id,
        anchor,
        proactive.silence_wake_retry_min_minutes,
        proactive.silence_wake_retry_max_minutes,
        "silence_wake",
    );
    let Some(bucket) = triggers::retry_bucket_index(now, anchor, interval) else {
        return false;
    };
    let final_p = (availability_base * proactive.probability_factor).clamp(0.0, 1.0);
    let (pass, roll) = triggers::retry_bucket_probability_passes(
        &candidate.character_id,
        anchor,
        bucket,
        "silence_wake",
        final_p,
    );
    if !pass {
        tracing::debug!(
            conversation_id = %candidate.id,
            bucket,
            interval,
            final_p,
            roll,
            "silence_wake: bucket miss"
        );
        return false;
    }
    tracing::debug!(
        conversation_id = %candidate.id,
        last_user_message_at = ?candidate.last_user_message_at,
        silence_after_hours = proactive.silence_after_hours,
        bucket,
        interval,
        "silence_wake eligible"
    );
    true
}

async fn evaluate_schedule_change(
    chat: &Arc<ChatService>,
    candidate: &conversation_queries::ProactiveCandidateRow,
    now: chrono::DateTime<Utc>,
    availability: crate::domain::Availability,
    schedule_change_probability: f64,
    probability_factor: f64,
) -> Result<(bool, ScheduleChangeExtras), AppError> {
    let mut extras = ScheduleChangeExtras::default();

    if candidate.status != crate::domain::ConversationStatus::Active {
        return Ok((false, extras));
    }
    if !availability.at_least_medium() {
        tracing::trace!(
            conversation_id = %candidate.id,
            availability = %availability,
            "schedule_change: availability too low"
        );
        return Ok((false, extras));
    }

    let Some(raw) = characters::find_schedule_json(&chat.db, &candidate.character_id).await?
    else {
        return Ok((false, extras));
    };
    let schedule = match schedule::Schedule::try_from_json_value(raw) {
        Ok(None) => return Ok((false, extras)),
        Ok(Some(schedule)) => schedule,
        Err(err) => {
            tracing::warn!(
                conversation_id = %candidate.id,
                character_id = %candidate.character_id,
                "schedule_change: invalid schedule_json: {err}"
            );
            return Ok((false, extras));
        }
    };

    let Some(status) = current_custom_slot(&schedule, now)? else {
        tracing::trace!(
            conversation_id = %candidate.id,
            "schedule_change: not in kind=custom slot"
        );
        return Ok((false, extras));
    };
    extras.current_activity = status.activity.clone();
    extras.previous_activity = status.previous_activity.clone();

    let cfg = &chat.proactive_config;
    if !in_schedule_change_window(
        &status,
        &candidate.character_id,
        cfg.schedule_change_window_min_mins,
        cfg.schedule_change_window_max_mins,
    ) {
        tracing::trace!(
            conversation_id = %candidate.id,
            activity = %status.activity,
            minutes_into_slot = status.minutes_into_slot,
            "schedule_change: outside lead-in window"
        );
        return Ok((false, extras));
    }

    let already = proactive_logs::count_trigger_between(
        &chat.db,
        &candidate.user_id,
        &candidate.character_id,
        TriggerType::ScheduleChange.as_str(),
        status.slot_started_at,
        now + chrono::Duration::seconds(1),
    )
    .await?;
    if already > 0 {
        tracing::trace!(
            conversation_id = %candidate.id,
            activity = %status.activity,
            "schedule_change: already sent for this slot"
        );
        return Ok((false, extras));
    }

    let base = chat.proactive_config.base_probability(availability);
    let (pass, final_p, roll) = triggers::schedule_change_probability_passes(
        &candidate.character_id,
        status.slot_started_at,
        schedule_change_probability,
        base,
        probability_factor,
    );
    if !pass {
        tracing::debug!(
            conversation_id = %candidate.id,
            activity = %status.activity,
            tendency = schedule_change_probability,
            base,
            probability_factor,
            final_p,
            roll,
            "schedule_change: personality probability miss (once per slot)"
        );
        return Ok((false, extras));
    }

    tracing::debug!(
        conversation_id = %candidate.id,
        activity = %status.activity,
        previous = ?status.previous_activity,
        minutes_into_slot = status.minutes_into_slot,
        tendency = schedule_change_probability,
        final_p,
        roll,
        "schedule_change eligible"
    );
    Ok((true, extras))
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
    pre_sleep_extras: &PreSleepExtras,
    re_engage_extras: &ReEngageExtras,
    schedule_change_extras: &ScheduleChangeExtras,
) -> Result<bool, AppError> {
    let turn_id = Uuid::new_v4().to_string();
    let special = if trigger == TriggerType::DailyGreeting {
        special_date_detail(greeting_extras)
    } else {
        None
    };
    let ask_user_busy_care =
        trigger == TriggerType::PreSleep && pre_sleep_extras.ask_user_busy_care;
    let schedule_change = if trigger == TriggerType::ScheduleChange {
        Some((
            schedule_change_extras.current_activity.as_str(),
            schedule_change_extras.previous_activity.as_deref(),
        ))
    } else {
        None
    };
    let re_engage_reason = if trigger == TriggerType::ReEngage {
        re_engage_extras.reason.map(|r| r.as_str())
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
        ask_user_busy_care,
        schedule_change,
        re_engage_reason,
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
