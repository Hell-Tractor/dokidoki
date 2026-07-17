//! `re_engage` 触发求值（char_busy / user_busy）。

use chrono::{DateTime, Utc};

use crate::db::queries::conversations as conversation_queries;
use crate::domain::persona::ProactiveConfig;

use super::{
    triggers::{is_char_busy_re_engage_ready, try_retry_bucket, user_busy_re_engage_probability},
    ReEngageReason, TriggerFire,
};

pub fn evaluate(
    candidate: &conversation_queries::ProactiveCandidateRow,
    now: DateTime<Utc>,
    proactive: &ProactiveConfig,
    availability_base: f64,
) -> Option<TriggerFire> {
    let character_id = candidate.character_id.as_str();
    let factor = proactive.probability_factor;

    if is_char_busy_re_engage_ready(candidate.status, candidate.activity_ends_at, now) {
        let anchor = candidate.activity_ends_at?;
        let final_p = (availability_base * factor).clamp(0.0, 1.0);
        let attempt = try_retry_bucket(
            character_id,
            now,
            anchor,
            proactive.re_engage_retry_min_minutes,
            proactive.re_engage_retry_max_minutes,
            "re_engage_char",
            final_p,
        )?;
        if !attempt.passed {
            tracing::debug!(
                conversation_id = %candidate.id,
                bucket = attempt.bucket,
                interval = attempt.interval,
                final_p = attempt.final_p,
                roll = attempt.roll,
                "re_engage: char_busy bucket miss"
            );
            return None;
        }
        tracing::debug!(
            conversation_id = %candidate.id,
            activity_ends_at = ?candidate.activity_ends_at,
            bucket = attempt.bucket,
            interval = attempt.interval,
            final_p = attempt.final_p,
            "re_engage eligible: paused_char_busy"
        );
        return Some(TriggerFire::ReEngage {
            reason: ReEngageReason::CharBusy,
        });
    }

    if let Some(curve_p) = user_busy_re_engage_probability(
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
            return None;
        }
        let anchor = candidate.paused_at?;
        let curve_start = anchor
            + chrono::Duration::minutes(i64::from(proactive.user_busy_reengage.min_delay_minutes));
        let final_p = (curve_p * availability_base * factor).clamp(0.0, 1.0);
        let attempt = try_retry_bucket(
            character_id,
            now,
            curve_start,
            proactive.re_engage_retry_min_minutes,
            proactive.re_engage_retry_max_minutes,
            "re_engage_user",
            final_p,
        )?;
        if !attempt.passed {
            tracing::debug!(
                conversation_id = %candidate.id,
                bucket = attempt.bucket,
                interval = attempt.interval,
                curve_p,
                final_p = attempt.final_p,
                roll = attempt.roll,
                "re_engage: user_busy bucket miss"
            );
            return None;
        }
        tracing::debug!(
            conversation_id = %candidate.id,
            curve_p,
            bucket = attempt.bucket,
            interval = attempt.interval,
            final_p = attempt.final_p,
            "re_engage eligible: paused_user_busy"
        );
        return Some(TriggerFire::ReEngage {
            reason: ReEngageReason::UserBusy,
        });
    }
    None
}
