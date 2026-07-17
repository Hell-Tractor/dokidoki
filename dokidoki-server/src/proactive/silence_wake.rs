//! `silence_wake` 触发求值。

use chrono::{DateTime, Utc};

use crate::db::queries::conversations as conversation_queries;
use crate::domain::{persona::ProactiveConfig, Availability};

use super::{
    triggers::{is_silence_wake_eligible, try_retry_bucket},
    TriggerFire,
};

pub fn evaluate(
    candidate: &conversation_queries::ProactiveCandidateRow,
    now: DateTime<Utc>,
    availability: Availability,
    proactive: &ProactiveConfig,
    availability_base: f64,
) -> Option<TriggerFire> {
    if !is_silence_wake_eligible(
        candidate.status,
        candidate.last_user_message_at,
        now,
        proactive.silence_after_hours,
        availability,
    ) {
        return None;
    }
    let last_user = candidate.last_user_message_at?;
    let silence_ms = (proactive.silence_after_hours * 3_600_000.0).round() as i64;
    let anchor = last_user + chrono::Duration::milliseconds(silence_ms);
    let final_p = (availability_base * proactive.probability_factor).clamp(0.0, 1.0);
    let attempt = try_retry_bucket(
        &candidate.character_id,
        now,
        anchor,
        proactive.silence_wake_retry_min_minutes,
        proactive.silence_wake_retry_max_minutes,
        "silence_wake",
        final_p,
    )?;
    if !attempt.passed {
        tracing::debug!(
            conversation_id = %candidate.id,
            bucket = attempt.bucket,
            interval = attempt.interval,
            final_p = attempt.final_p,
            roll = attempt.roll,
            "silence_wake: bucket miss"
        );
        return None;
    }
    tracing::debug!(
        conversation_id = %candidate.id,
        last_user_message_at = ?candidate.last_user_message_at,
        silence_after_hours = proactive.silence_after_hours,
        bucket = attempt.bucket,
        interval = attempt.interval,
        "silence_wake eligible"
    );
    Some(TriggerFire::SilenceWake)
}
