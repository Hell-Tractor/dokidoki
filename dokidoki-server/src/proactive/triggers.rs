//! 主动消息触发器。

use chrono::{DateTime, Datelike, NaiveDate, Utc};

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
    pub status: &'a str,
    pub availability: &'a str,
    /// 已在起床段问候窗内，且本会话角色今日尚未发过 `daily_greeting`。
    pub daily_greeting_eligible: bool,
    /// `paused` 且超过 `re_engage_after_minutes`，availability ≥ medium。
    pub re_engage_eligible: bool,
}

/// 按优先级取一条：daily_greeting → re_engage → silence_wake → mood_followup → schedule_change。
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
        TriggerType::SilenceWake => false,
        TriggerType::MoodFollowup => false,
        TriggerType::ScheduleChange => false,
    }
}

/// `high` / `medium` 视为 ≥ medium；其余（含 `low`）不通过。
pub fn availability_at_least_medium(availability: &str) -> bool {
    matches!(availability, "high" | "medium")
}

/// 话题重启条件：`status=paused`，已过等候分钟数，availability ≥ medium。
pub fn is_re_engage_eligible(
    status: &str,
    paused_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
    after_minutes: u32,
    availability: &str,
) -> bool {
    if status != "paused" {
        return false;
    }
    if !availability_at_least_medium(availability) {
        return false;
    }
    let Some(paused_at) = paused_at else {
        return false;
    };
    let elapsed = now.signed_duration_since(paused_at);
    elapsed.num_minutes() >= i64::from(after_minutes)
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
    use chrono::{NaiveDate, TimeZone};

    fn ctx(daily: bool, re_engage: bool) -> TriggerContext<'static> {
        TriggerContext {
            conversation_id: "c1",
            status: "paused",
            availability: "medium",
            daily_greeting_eligible: daily,
            re_engage_eligible: re_engage,
        }
    }

    #[test]
    fn selects_daily_greeting_over_re_engage() {
        assert_eq!(
            select_trigger(&ctx(true, true)),
            Some(TriggerType::DailyGreeting)
        );
        assert_eq!(select_trigger(&ctx(false, true)), Some(TriggerType::ReEngage));
        assert_eq!(select_trigger(&ctx(false, false)), None);
    }

    #[test]
    fn re_engage_requires_paused_elapsed_and_availability() {
        let paused_at = Utc.with_ymd_and_hms(2026, 7, 14, 10, 0, 0).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 7, 14, 12, 0, 0).unwrap(); // +120 min

        assert!(is_re_engage_eligible("paused", Some(paused_at), now, 120, "medium"));
        assert!(is_re_engage_eligible("paused", Some(paused_at), now, 120, "high"));
        assert!(!is_re_engage_eligible("paused", Some(paused_at), now, 120, "low"));
        assert!(!is_re_engage_eligible("active", Some(paused_at), now, 120, "medium"));
        assert!(!is_re_engage_eligible("paused", None, now, 120, "medium"));
        assert!(!is_re_engage_eligible(
            "paused",
            Some(paused_at),
            Utc.with_ymd_and_hms(2026, 7, 14, 11, 59, 0).unwrap(),
            120,
            "medium"
        ));
    }

    #[test]
    fn birthday_matches_user_local_month_day() {
        // 2026-07-10 20:00 UTC = 2026-07-11 04:00 Asia/Shanghai
        let now = Utc.with_ymd_and_hms(2026, 7, 10, 20, 0, 0).unwrap();
        let bday = NaiveDate::from_ymd_opt(2000, 7, 11).unwrap();
        assert!(is_birthday_today(Some(bday), now, "Asia/Shanghai").unwrap());
        assert!(!is_birthday_today(Some(bday), now, "UTC").unwrap());
    }
}
