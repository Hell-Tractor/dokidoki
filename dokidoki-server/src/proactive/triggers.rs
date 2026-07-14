//! 主动消息触发器。

use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};

use crate::domain::persona::UserBusyReengage;
use crate::domain::{Availability, ConversationStatus};

/// 触发类型（与 `proactive_logs.trigger_type` / Prompt `{proactive_trigger}` 对齐）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerType {
    DailyGreeting,
    ReEngage,
    SilenceWake,
    MoodFollowup,
    ScheduleChange,
}

impl TriggerType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DailyGreeting => "daily_greeting",
            Self::ReEngage => "re_engage",
            Self::SilenceWake => "silence_wake",
            Self::MoodFollowup => "mood_followup",
            Self::ScheduleChange => "schedule_change",
        }
    }
}

/// 每日问候附加语境（合并 special_date，不单独触发）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DailyGreetingExtras {
    /// 用户时区「今天」是否为其生日。
    pub is_user_birthday: bool,
    pub user_birthday: Option<NaiveDate>,
    /// 公共节日名（方案待评估；Nager 已暂缓）。
    pub holiday_names: Vec<String>,
}

impl DailyGreetingExtras {
    pub fn has_special_date(&self) -> bool {
        self.is_user_birthday || !self.holiday_names.is_empty()
    }
}

/// 触发器求值上下文（字段随各类触发落地逐步补齐）。
#[derive(Debug, Clone)]
pub struct TriggerContext<'a> {
    pub conversation_id: &'a str,
    pub status: ConversationStatus,
    pub availability: Availability,
    /// 已在起床段问候窗内，且本会话角色今日尚未发过 `daily_greeting`。
    pub daily_greeting_eligible: bool,
    /// `paused_char_busy` / `paused_user_busy` 且各自时机条件满足。
    pub re_engage_eligible: bool,
    /// 仅 `paused`：距用户末条超过 `silence_after_hours`。
    pub silence_wake_eligible: bool,
}

/// 按优先级取一条：daily_greeting → re_engage → silence_wake → …
pub fn select_trigger(ctx: &TriggerContext<'_>) -> Option<TriggerType> {
    for candidate in [
        TriggerType::DailyGreeting,
        TriggerType::ReEngage,
        TriggerType::SilenceWake,
        TriggerType::MoodFollowup,
        TriggerType::ScheduleChange,
    ] {
        if evaluate(candidate, ctx) {
            return Some(candidate);
        }
    }
    None
}

fn evaluate(trigger: TriggerType, ctx: &TriggerContext<'_>) -> bool {
    match trigger {
        TriggerType::DailyGreeting => ctx.daily_greeting_eligible,
        TriggerType::ReEngage => ctx.re_engage_eligible,
        TriggerType::SilenceWake => ctx.silence_wake_eligible,
        TriggerType::MoodFollowup => false,
        TriggerType::ScheduleChange => false,
    }
}

/// 角色忙完：`PausedCharBusy` 且 `now ≥ activity_ends_at`（无 ends_at 则不可）。
pub fn is_char_busy_re_engage_ready(
    status: ConversationStatus,
    activity_ends_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> bool {
    if status != ConversationStatus::PausedCharBusy {
        return false;
    }
    let Some(ends_at) = activity_ends_at else {
        return false;
    };
    now >= ends_at
}

/// 用户忙重启：返回曲线概率 \(P(t)\)；不满足状态时返回 `None`。
pub fn user_busy_re_engage_probability(
    status: ConversationStatus,
    paused_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
    curve: &UserBusyReengage,
) -> Option<f64> {
    if status != ConversationStatus::PausedUserBusy {
        return None;
    }
    let Some(paused_at) = paused_at else {
        return None;
    };
    let elapsed_secs = now.signed_duration_since(paused_at).num_seconds().max(0) as f64;
    let elapsed_minutes = elapsed_secs / 60.0;
    Some(curve.probability(elapsed_minutes))
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
    if !(silence_after_hours > 0.0) {
        return false;
    }
    let Some(last_user_message_at) = last_user_message_at else {
        return false;
    };
    let Some(threshold) = duration_from_hours(silence_after_hours) else {
        return false;
    };
    now.signed_duration_since(last_user_message_at) >= threshold
}

fn duration_from_hours(hours: f64) -> Option<Duration> {
    let millis = (hours * 3_600_000.0).round();
    if !millis.is_finite() || millis > i64::MAX as f64 || millis < 0.0 {
        return None;
    }
    Some(Duration::milliseconds(millis as i64))
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

    fn ctx(daily: bool, re_engage: bool, silence: bool) -> TriggerContext<'static> {
        TriggerContext {
            conversation_id: "c1",
            status: ConversationStatus::Paused,
            availability: Availability::Medium,
            daily_greeting_eligible: daily,
            re_engage_eligible: re_engage,
            silence_wake_eligible: silence,
        }
    }

    #[test]
    fn selects_by_priority() {
        assert_eq!(
            select_trigger(&ctx(true, true, true)),
            Some(TriggerType::DailyGreeting)
        );
        assert_eq!(
            select_trigger(&ctx(false, true, true)),
            Some(TriggerType::ReEngage)
        );
        assert_eq!(
            select_trigger(&ctx(false, false, true)),
            Some(TriggerType::SilenceWake)
        );
        assert_eq!(select_trigger(&ctx(false, false, false)), None);
    }

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
                &curve
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
                paused + Duration::hours(2),
                &curve
            ),
            None
        );
    }

    #[test]
    fn silence_wake_only_from_paused() {
        let last = Utc.with_ymd_and_hms(2026, 7, 14, 0, 0, 0).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 7, 14, 8, 0, 0).unwrap();
        assert!(is_silence_wake_eligible(
            ConversationStatus::Paused,
            Some(last),
            now,
            8.0,
            Availability::Medium
        ));
        assert!(!is_silence_wake_eligible(
            ConversationStatus::PausedCharBusy,
            Some(last),
            now,
            8.0,
            Availability::Medium
        ));
        assert!(!is_silence_wake_eligible(
            ConversationStatus::Active,
            Some(last),
            now,
            8.0,
            Availability::Medium
        ));
    }

    #[test]
    fn birthday_matches_user_local_month_day() {
        let now = Utc.with_ymd_and_hms(2026, 7, 10, 20, 0, 0).unwrap();
        let bday = NaiveDate::from_ymd_opt(2000, 7, 11).unwrap();
        assert!(is_birthday_today(Some(bday), now, "Asia/Shanghai").unwrap());
        assert!(!is_birthday_today(Some(bday), now, "UTC").unwrap());
    }
}
