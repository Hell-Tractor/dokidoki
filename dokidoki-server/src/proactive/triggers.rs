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
    /// 公共节日名（Nager 后续接入）。
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
        TriggerType::ReEngage => false,
        TriggerType::SilenceWake => false,
        TriggerType::MoodFollowup => false,
        TriggerType::ScheduleChange => false,
    }
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

    fn ctx(eligible: bool) -> TriggerContext<'static> {
        TriggerContext {
            conversation_id: "c1",
            status: "active",
            availability: "high",
            daily_greeting_eligible: eligible,
        }
    }

    #[test]
    fn selects_daily_greeting_when_eligible() {
        assert_eq!(select_trigger(&ctx(true)), Some(TriggerType::DailyGreeting));
        assert_eq!(select_trigger(&ctx(false)), None);
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
