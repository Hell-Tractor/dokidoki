//! `daily_greeting` 触发求值（含 special_date 注入）。

use chrono::{DateTime, NaiveDate, Utc};

use crate::{
    chat::ChatService,
    db::queries::{conversations as conversation_queries, proactive_logs, users},
    error::AppError,
    schedule::{at_daily_greeting_fire_time, current_wakeup_slot, Schedule},
};

use super::{char_schedule, triggers, TriggerFire};

pub async fn evaluate(
    chat: &ChatService,
    candidate: &conversation_queries::ProactiveCandidateRow,
    now: DateTime<Utc>,
    schedule: Option<&Schedule>,
) -> Result<Option<TriggerFire>, AppError> {
    let user = users::find_by_id(&chat.db, &candidate.user_id)
        .await?
        .ok_or_else(|| AppError::not_found("用户不存在"))?;
    let is_user_birthday =
        triggers::is_birthday_today(user.birthday, now, &candidate.timezone)?;

    let Some(schedule) = schedule else {
        tracing::trace!(
            conversation_id = %candidate.id,
            character_id = %candidate.character_id,
            "daily_greeting: character has no schedule_json"
        );
        return Ok(None);
    };

    let Some(wakeup) = current_wakeup_slot(schedule, now)? else {
        tracing::trace!(
            conversation_id = %candidate.id,
            character_id = %candidate.character_id,
            "daily_greeting: not in kind=wake slot"
        );
        return Ok(None);
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
        return Ok(None);
    }

    // 「每角色每日一次」按角色日程时区自然日统计。
    let (char_day_start, char_day_end) =
        char_schedule::character_local_day_bounds(&schedule.timezone, wakeup.local_date)?;
    let already = proactive_logs::count_trigger_between(
        &chat.db,
        &candidate.user_id,
        &candidate.character_id,
        "daily_greeting",
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
        return Ok(None);
    }

    let special_date_detail = special_date_detail(is_user_birthday, user.birthday, &[]);
    tracing::debug!(
        conversation_id = %candidate.id,
        activity = %wakeup.activity,
        minutes_into_slot = wakeup.minutes_into_slot,
        is_user_birthday,
        "daily_greeting eligible"
    );
    Ok(Some(TriggerFire::DailyGreeting {
        special_date_detail,
    }))
}

fn special_date_detail(
    is_user_birthday: bool,
    user_birthday: Option<NaiveDate>,
    holiday_names: &[String],
) -> Option<String> {
    if !is_user_birthday && holiday_names.is_empty() {
        return None;
    }
    let mut parts = Vec::new();
    if is_user_birthday {
        if let Some(date) = user_birthday {
            parts.push(format!("对方生日（{}）", date.format("%m-%d")));
        } else {
            parts.push("对方生日".into());
        }
    }
    for name in holiday_names {
        parts.push(name.clone());
    }
    Some(parts.join("、"))
}
