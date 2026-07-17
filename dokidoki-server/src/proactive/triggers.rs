//! 主动消息触发器：时机判定与抽样工具。

use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};

use crate::domain::persona::UserBusyReengage;
use crate::domain::{Availability, ConversationStatus};
use crate::prompt::ProactiveScene;

/// 求值命中结果（与 Prompt 场景同一类型，避免 TriggerType / Extras 双轨）。
pub type TriggerFire = ProactiveScene;

pub use crate::prompt::ReEngageReason;

/// 角色忙完：`PausedCharBusy` 且 `now ≥ activity_ends_at`（无 ends_at 则不可）。
pub fn is_char_busy_re_engage_ready(
    status: ConversationStatus,
    activity_ends_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> bool {
    if status != ConversationStatus::PausedCharBusy {
        return false;
    }
    activity_ends_at.is_some_and(|ends| now >= ends)
}

/// `paused_user_busy` 下按曲线得到 \(P(t)\)；非该状态返回 `None`。
pub fn user_busy_re_engage_probability(
    status: ConversationStatus,
    paused_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
    curve: &UserBusyReengage,
) -> Option<f64> {
    if status != ConversationStatus::PausedUserBusy {
        return None;
    }
    let paused_at = paused_at?;
    let elapsed = (now - paused_at).num_minutes() as f64;
    Some(curve.probability(elapsed))
}

/// 沉默唤醒：仅 `Paused`，距用户末条消息 ≥ `silence_after_hours`，availability ≥ medium。
pub fn is_silence_wake_eligible(
    status: ConversationStatus,
    last_user_message_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
    silence_after_hours: f64,
    availability: Availability,
) -> bool {
    if status != ConversationStatus::Paused {
        return false;
    }
    if !availability.at_least_medium() {
        return false;
    }
    let Some(last) = last_user_message_at else {
        return false;
    };
    let Some(threshold) = duration_from_hours(silence_after_hours) else {
        return false;
    };
    now >= last + threshold
}

/// 日程切换：对 `(character_id, slot_started_at)` 确定性抽样一次。
pub fn schedule_change_probability_passes(
    character_id: &str,
    slot_started_at: DateTime<Utc>,
    tendency: f64,
    availability_base: f64,
    probability_factor: f64,
) -> (bool, f64, f64) {
    let final_p =
        (tendency.max(0.0) * availability_base * probability_factor.max(0.0)).clamp(0.0, 1.0);
    let roll = hash_unit(&[
        character_id.as_bytes(),
        &slot_started_at.timestamp().to_le_bytes(),
        b"schedule_change",
    ]);
    (roll < final_p, final_p, roll)
}

/// `re_engage` / `silence_wake` 重试间隔（分钟）：对 character + anchor 均匀取 `[min,max]`。
pub fn retry_interval_mins(
    character_id: &str,
    anchor: DateTime<Utc>,
    min_minutes: u32,
    max_minutes: u32,
    salt: &str,
) -> u32 {
    let lo = min_minutes.min(max_minutes).max(1);
    let hi = min_minutes.max(max_minutes).max(1);
    if lo == hi {
        return lo;
    }
    let unit = hash_unit(&[
        character_id.as_bytes(),
        &anchor.timestamp().to_le_bytes(),
        salt.as_bytes(),
        b"interval",
    ]);
    lo + ((unit * f64::from(hi - lo + 1)) as u32).min(hi - lo)
}

/// 自 `anchor` 起第几个重试桶（0-based）；`now < anchor` 则 `None`。
pub fn retry_bucket_index(
    now: DateTime<Utc>,
    anchor: DateTime<Utc>,
    interval_mins: u32,
) -> Option<u32> {
    if now < anchor || interval_mins == 0 {
        return None;
    }
    let elapsed = (now - anchor).num_minutes().max(0) as u32;
    Some(elapsed / interval_mins)
}

/// 对 `(character, anchor, bucket, salt)` 做一次确定性概率抽样。
pub fn retry_bucket_probability_passes(
    character_id: &str,
    anchor: DateTime<Utc>,
    bucket: u32,
    salt: &str,
    final_p: f64,
) -> (bool, f64) {
    let roll = hash_unit(&[
        character_id.as_bytes(),
        &anchor.timestamp().to_le_bytes(),
        &bucket.to_le_bytes(),
        salt.as_bytes(),
        b"roll",
    ]);
    (roll < final_p.clamp(0.0, 1.0), roll)
}

/// 一次重试桶抽样结果（`passed == false` 表示本桶未命中）。
#[derive(Debug, Clone, Copy)]
pub struct RetryBucketAttempt {
    pub bucket: u32,
    pub interval: u32,
    pub final_p: f64,
    pub roll: f64,
    pub passed: bool,
}

/// 合成：算 interval → bucket → 确定性抽样。`now < anchor` 或 interval 无效时返回 `None`。
pub fn try_retry_bucket(
    character_id: &str,
    now: DateTime<Utc>,
    anchor: DateTime<Utc>,
    min_minutes: u32,
    max_minutes: u32,
    salt: &str,
    final_p: f64,
) -> Option<RetryBucketAttempt> {
    let interval = retry_interval_mins(character_id, anchor, min_minutes, max_minutes, salt);
    let bucket = retry_bucket_index(now, anchor, interval)?;
    let final_p = final_p.clamp(0.0, 1.0);
    let (passed, roll) =
        retry_bucket_probability_passes(character_id, anchor, bucket, salt, final_p);
    Some(RetryBucketAttempt {
        bucket,
        interval,
        final_p,
        roll,
        passed,
    })
}

fn hash_unit(parts: &[&[u8]]) -> f64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    for part in parts {
        part.hash(&mut hasher);
    }
    (hasher.finish() as f64) / (u64::MAX as f64)
}

fn duration_from_hours(hours: f64) -> Option<Duration> {
    if !hours.is_finite() || hours < 0.0 {
        return None;
    }
    Some(Duration::milliseconds((hours * 3_600_000.0).round() as i64))
}

/// 用户生日是否落在其本地「今天」（月-日）。
pub fn is_birthday_today(
    birthday: Option<NaiveDate>,
    now: DateTime<Utc>,
    user_timezone: &str,
) -> Result<bool, crate::error::AppError> {
    let Some(birthday) = birthday else {
        return Ok(false);
    };
    let local = crate::time::local_time(now, user_timezone)?;
    Ok(local.date().month() == birthday.month() && local.date().day() == birthday.day())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn char_busy_re_engage_waits_for_activity_end() {
        let ends = Utc.with_ymd_and_hms(2026, 7, 14, 12, 0, 0).unwrap();
        assert!(is_char_busy_re_engage_ready(
            ConversationStatus::PausedCharBusy,
            Some(ends),
            Utc.with_ymd_and_hms(2026, 7, 14, 12, 0, 0).unwrap()
        ));
        assert!(!is_char_busy_re_engage_ready(
            ConversationStatus::PausedCharBusy,
            Some(ends),
            Utc.with_ymd_and_hms(2026, 7, 14, 11, 59, 0).unwrap()
        ));
        assert!(!is_char_busy_re_engage_ready(
            ConversationStatus::Paused,
            Some(ends),
            ends
        ));
        assert!(!is_char_busy_re_engage_ready(
            ConversationStatus::PausedCharBusy,
            None,
            ends
        ));
    }

    #[test]
    fn user_busy_curve_probability() {
        let curve = UserBusyReengage {
            min_delay_minutes: 30,
            ramp_minutes: 90,
            peak_probability: 0.6,
        };
        let paused = Utc.with_ymd_and_hms(2026, 7, 14, 10, 0, 0).unwrap();
        assert_eq!(
            user_busy_re_engage_probability(
                ConversationStatus::PausedUserBusy,
                Some(paused),
                Utc.with_ymd_and_hms(2026, 7, 14, 10, 20, 0).unwrap(),
                &curve,
            ),
            Some(0.0)
        );
        let mid = user_busy_re_engage_probability(
            ConversationStatus::PausedUserBusy,
            Some(paused),
            Utc.with_ymd_and_hms(2026, 7, 14, 11, 15, 0).unwrap(),
            &curve,
        )
        .unwrap();
        assert!((mid - 0.3).abs() < 1e-9);
        assert_eq!(
            user_busy_re_engage_probability(
                ConversationStatus::Paused,
                Some(paused),
                Utc.with_ymd_and_hms(2026, 7, 14, 12, 0, 0).unwrap(),
                &curve,
            ),
            None
        );
    }

    #[test]
    fn silence_wake_requires_paused_and_gap() {
        let last = Utc.with_ymd_and_hms(2026, 7, 14, 8, 0, 0).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 7, 14, 16, 0, 0).unwrap();
        assert!(is_silence_wake_eligible(
            ConversationStatus::Paused,
            Some(last),
            now,
            4.0,
            Availability::Medium,
        ));
        assert!(!is_silence_wake_eligible(
            ConversationStatus::PausedCharBusy,
            Some(last),
            now,
            4.0,
            Availability::High,
        ));
        assert!(!is_silence_wake_eligible(
            ConversationStatus::Paused,
            Some(last),
            Utc.with_ymd_and_hms(2026, 7, 14, 10, 0, 0).unwrap(),
            4.0,
            Availability::High,
        ));
    }

    #[test]
    fn retry_bucket_index_and_stable_interval() {
        let anchor = Utc.with_ymd_and_hms(2026, 7, 14, 10, 0, 0).unwrap();
        let interval = retry_interval_mins("c1", anchor, 15, 45, "re_engage_char");
        assert!((15..=45).contains(&interval));
        assert_eq!(
            retry_interval_mins("c1", anchor, 15, 45, "re_engage_char"),
            interval
        );
        assert_eq!(
            retry_bucket_index(
                anchor + Duration::minutes(i64::from(interval) - 1),
                anchor,
                interval
            ),
            Some(0)
        );
        assert_eq!(
            retry_bucket_index(
                anchor + Duration::minutes(i64::from(interval)),
                anchor,
                interval
            ),
            Some(1)
        );
        assert!(retry_bucket_index(anchor - Duration::minutes(1), anchor, interval).is_none());
    }

    #[test]
    fn try_retry_bucket_none_before_anchor() {
        let anchor = Utc.with_ymd_and_hms(2026, 7, 14, 10, 0, 0).unwrap();
        assert!(try_retry_bucket(
            "c1",
            anchor - Duration::minutes(1),
            anchor,
            15,
            45,
            "re_engage_char",
            1.0,
        )
        .is_none());
        let attempt = try_retry_bucket("c1", anchor, anchor, 15, 45, "re_engage_char", 1.0).unwrap();
        assert_eq!(attempt.bucket, 0);
        assert!(attempt.passed); // final_p = 1.0
    }

    #[test]
    fn trigger_fire_as_str() {
        assert_eq!(TriggerFire::SilenceWake.as_str(), "silence_wake");
        assert_eq!(ReEngageReason::CharBusy.as_str(), "char_busy");
    }
}
