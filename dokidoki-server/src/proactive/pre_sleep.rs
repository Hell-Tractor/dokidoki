//! `pre_sleep` 触发求值。

use chrono::{DateTime, Utc};

use crate::{
    chat::ChatService,
    db::queries::{conversations as conversation_queries, proactive_logs},
    error::AppError,
    schedule::{at_pre_sleep_fire_time, upcoming_pre_sleep, Schedule},
};

use super::{char_schedule, TriggerFire};

pub async fn evaluate(
    chat: &ChatService,
    candidate: &conversation_queries::ProactiveCandidateRow,
    now: DateTime<Utc>,
    schedule: Option<&Schedule>,
) -> Result<Option<TriggerFire>, AppError> {
    let ask_user_busy_care =
        candidate.status == crate::domain::ConversationStatus::PausedCharBusy;

    let Some(schedule) = schedule else {
        return Ok(None);
    };

    let Some(status) = upcoming_pre_sleep(schedule, now)? else {
        tracing::trace!(
            conversation_id = %candidate.id,
            character_id = %candidate.character_id,
            "pre_sleep: no upcoming sleep window"
        );
        return Ok(None);
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
        return Ok(None);
    }

    let (char_day_start, char_day_end) =
        char_schedule::character_local_day_bounds(&schedule.timezone, status.sleep_local_date)?;
    let already = proactive_logs::count_trigger_between(
        &chat.db,
        &candidate.user_id,
        &candidate.character_id,
        "pre_sleep",
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
        return Ok(None);
    }

    tracing::debug!(
        conversation_id = %candidate.id,
        minutes_until_sleep = status.minutes_until_sleep,
        ask_user_busy_care,
        "pre_sleep eligible"
    );
    Ok(Some(TriggerFire::PreSleep { ask_user_busy_care }))
}
