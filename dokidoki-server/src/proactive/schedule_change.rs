//! `schedule_change` 触发求值。

use chrono::{DateTime, Utc};

use crate::{
    chat::ChatService,
    db::queries::{conversations as conversation_queries, proactive_logs},
    domain::Availability,
    error::AppError,
    schedule::{current_custom_slot, in_schedule_change_window, Schedule},
};

use super::{triggers, TriggerFire};

pub async fn evaluate(
    chat: &ChatService,
    candidate: &conversation_queries::ProactiveCandidateRow,
    now: DateTime<Utc>,
    schedule: Option<&Schedule>,
    availability: Availability,
    schedule_change_probability: f64,
    probability_factor: f64,
) -> Result<Option<TriggerFire>, AppError> {
    if candidate.status != crate::domain::ConversationStatus::Active {
        return Ok(None);
    }
    if !availability.at_least_medium() {
        tracing::trace!(
            conversation_id = %candidate.id,
            availability = %availability,
            "schedule_change: availability too low"
        );
        return Ok(None);
    }

    let Some(schedule) = schedule else {
        return Ok(None);
    };

    let Some(status) = current_custom_slot(schedule, now)? else {
        tracing::trace!(
            conversation_id = %candidate.id,
            "schedule_change: not in kind=custom slot"
        );
        return Ok(None);
    };

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
        return Ok(None);
    }

    let already = proactive_logs::count_trigger_between(
        &chat.db,
        &candidate.user_id,
        &candidate.character_id,
        "schedule_change",
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
        return Ok(None);
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
        return Ok(None);
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
    Ok(Some(TriggerFire::ScheduleChange {
        current_activity: status.activity,
        previous_activity: status.previous_activity,
    }))
}
