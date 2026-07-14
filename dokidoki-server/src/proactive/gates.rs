//! 主动消息闸门：勿扰、日上限、availability × probability_factor。

use chrono::{DateTime, NaiveTime, Utc};

use crate::{
    config::Proactive,
    domain::Availability,
    time::{is_in_dnd_window, user_day_bounds},
    utils::UnitRng,
};

/// 是否通过概率闸门：`random_unit < base(availability) * probability_factor`。
pub fn passes_probability_gate(
    config: &Proactive,
    availability: Availability,
    probability_factor: f64,
    rng: &mut impl UnitRng,
) -> bool {
    let factor = probability_factor.max(0.0);
    let threshold = (config.base_probability(availability) * factor).clamp(0.0, 1.0);
    if threshold <= 0.0 {
        return false;
    }
    rng.next_unit() < threshold
}

/// 双方勿扰边界齐全且当前落在窗口内则拦截。
pub fn is_blocked_by_dnd(
    now: DateTime<Utc>,
    timezone: &str,
    dnd_start: Option<NaiveTime>,
    dnd_end: Option<NaiveTime>,
) -> Result<bool, crate::error::AppError> {
    match (dnd_start, dnd_end) {
        (Some(start), Some(end)) => is_in_dnd_window(now, timezone, start, end),
        _ => Ok(false),
    }
}

/// 当日已发送数是否已达用户上限（按用户时区自然日）。
pub fn is_at_daily_cap(
    sent_today: i64,
    max_proactive_per_day: i32,
) -> bool {
    max_proactive_per_day <= 0 || sent_today >= i64::from(max_proactive_per_day)
}

pub fn day_bounds_for_user(
    now: DateTime<Utc>,
    timezone: &str,
) -> Result<(DateTime<Utc>, DateTime<Utc>), crate::error::AppError> {
    user_day_bounds(now, timezone)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveTime, TimeZone};
    use crate::domain::Availability;
    use crate::utils::ScriptedRng;

    fn test_proactive_config() -> Proactive {
        Proactive {
            default_max_per_day: 20,
            availability_high: 0.45,
            availability_medium: 0.25,
            availability_low: 0.05,
            daily_greeting_window_min_mins: 30,
            daily_greeting_window_max_mins: 60,
        }
    }

    #[test]
    fn dnd_blocks_when_local_in_window() {
        let now = Utc.with_ymd_and_hms(2026, 7, 10, 15, 30, 0).unwrap(); // 23:30 Shanghai
        let start = NaiveTime::from_hms_opt(23, 0, 0).unwrap();
        let end = NaiveTime::from_hms_opt(7, 0, 0).unwrap();
        assert!(is_blocked_by_dnd(now, "Asia/Shanghai", Some(start), Some(end)).unwrap());
    }

    #[test]
    fn dnd_skips_when_boundaries_incomplete() {
        let now = Utc::now();
        let start = NaiveTime::from_hms_opt(23, 0, 0).unwrap();
        assert!(!is_blocked_by_dnd(now, "Asia/Shanghai", Some(start), None).unwrap());
    }

    #[test]
    fn daily_cap_compares_against_limit() {
        assert!(!is_at_daily_cap(5, 20));
        assert!(is_at_daily_cap(20, 20));
        assert!(is_at_daily_cap(0, 0));
    }

    #[test]
    fn probability_uses_availability_and_factor() {
        let config = test_proactive_config();
        // high base 0.45 * 1.0 = 0.45; unit 0.4 → pass; 0.5 → fail
        assert!(passes_probability_gate(
            &config,
            Availability::High,
            1.0,
            &mut ScriptedRng::new(vec![0.4])
        ));
        assert!(!passes_probability_gate(
            &config,
            Availability::High,
            1.0,
            &mut ScriptedRng::new(vec![0.5])
        ));
        // factor 0 → never
        assert!(!passes_probability_gate(
            &config,
            Availability::High,
            0.0,
            &mut ScriptedRng::new(vec![0.0])
        ));
    }

    #[test]
    fn base_probability_reads_from_config() {
        let config = Proactive {
            default_max_per_day: 20,
            availability_high: 0.9,
            availability_medium: 0.1,
            availability_low: 0.0,
            daily_greeting_window_min_mins: 30,
            daily_greeting_window_max_mins: 60,
        };
        assert_eq!(config.base_probability(Availability::High), 0.9);
        assert_eq!(config.base_probability(Availability::Medium), 0.1);
        assert_eq!(config.base_probability(Availability::Low), 0.0);
    }
}
