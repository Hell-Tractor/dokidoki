//! UTC 存储与用户时区语义辅助函数。
//!
//! - 绝对时刻：一律 `DateTime<Utc>`，API 序列化为 RFC 3339（带 `Z`）。
//! - 日历日期：一律 `NaiveDate`（如 `birthday`），不含时区。
//! - 角色日程：`characters.schedule_json.timezone`（IANA）。
//! - 用户本地语义（勿扰、日上限、生日）：`users.timezone`（IANA）。

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;

use crate::error::AppError;

pub fn parse_timezone(tz: &str) -> Result<Tz, AppError> {
    tz.parse::<Tz>()
        .map_err(|_| AppError::bad_request(format!("无效的时区: {tz}")))
}

pub fn is_valid_timezone(tz: &str) -> bool {
    tz.parse::<Tz>().is_ok()
}

pub fn local_time(now: DateTime<Utc>, tz: &str) -> Result<NaiveDateTime, AppError> {
    let tz = parse_timezone(tz)?;
    Ok(now.with_timezone(&tz).naive_local())
}

/// 返回 `now` 在用户时区自然日的 `[start, end)`（UTC 半开区间）。
pub fn user_day_bounds(
    now: DateTime<Utc>,
    tz: &str,
) -> Result<(DateTime<Utc>, DateTime<Utc>), AppError> {
    let tz = parse_timezone(tz)?;
    let local = now.with_timezone(&tz);
    let local_start = local.date_naive().and_hms_opt(0, 0, 0).unwrap();
    let local_end = local_start + chrono::Duration::days(1);
    Ok((
        tz.from_local_datetime(&local_start)
            .single()
            .ok_or_else(|| AppError::bad_request("无效的本地日界线"))?
            .with_timezone(&Utc),
        tz.from_local_datetime(&local_end)
            .single()
            .ok_or_else(|| AppError::bad_request("无效的本地日界线"))?
            .with_timezone(&Utc),
    ))
}

/// 判断 `date` 是否为用户时区日历上的「今天」。
pub fn is_user_today(date: NaiveDate, now: DateTime<Utc>, tz: &str) -> Result<bool, AppError> {
    Ok(date == local_time(now, tz)?.date())
}

/// 判断 `now` 是否落在用户本地勿扰窗口内。`start > end` 表示跨越本地午夜。
pub fn is_in_dnd_window(
    now: DateTime<Utc>,
    tz: &str,
    start: NaiveTime,
    end: NaiveTime,
) -> Result<bool, AppError> {
    let t = local_time(now, tz)?.time();
    Ok(if start <= end {
        t >= start && t < end
    } else {
        t >= start || t < end
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc_at(h: u32, m: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 7, 10, h, m, 0).unwrap()
    }

    fn local_time(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }

    #[test]
    fn parse_timezone_accepts_iana_names() {
        assert!(parse_timezone("Asia/Shanghai").is_ok());
        assert!(parse_timezone("UTC").is_ok());
        assert!(parse_timezone("Not/AZone").is_err());
    }

    #[test]
    fn user_day_bounds_use_local_midnight() {
        // 2026-07-10 20:00 UTC = 2026-07-11 04:00 Asia/Shanghai
        let now = utc_at(20, 0);
        let (start, end) = user_day_bounds(now, "Asia/Shanghai").unwrap();
        // Shanghai day 2026-07-11 starts at 2026-07-10 16:00 UTC
        assert_eq!(start, Utc.with_ymd_and_hms(2026, 7, 10, 16, 0, 0).unwrap());
        assert_eq!(end, Utc.with_ymd_and_hms(2026, 7, 11, 16, 0, 0).unwrap());
    }

    #[test]
    fn is_in_dnd_window_uses_user_local_clock() {
        // 15:30 UTC = 23:30 Asia/Shanghai
        let now = utc_at(15, 30);
        assert!(is_in_dnd_window(
            now,
            "Asia/Shanghai",
            local_time(23, 0),
            local_time(23, 59),
        )
        .unwrap());
        assert!(!is_in_dnd_window(
            now,
            "Asia/Shanghai",
            local_time(0, 0),
            local_time(23, 0),
        )
        .unwrap());
    }

    #[test]
    fn is_user_today_follows_user_calendar() {
        let now = utc_at(20, 0); // still Jul 10 UTC, Jul 11 in Shanghai
        assert!(!is_user_today(
            NaiveDate::from_ymd_opt(2026, 7, 11).unwrap(),
            now,
            "UTC",
        )
        .unwrap());
        assert!(is_user_today(
            NaiveDate::from_ymd_opt(2026, 7, 11).unwrap(),
            now,
            "Asia/Shanghai",
        )
        .unwrap());
    }
}
