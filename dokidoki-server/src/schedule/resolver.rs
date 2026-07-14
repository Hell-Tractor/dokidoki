use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use chrono::{DateTime, Datelike, NaiveDate, NaiveTime, TimeZone, Timelike, Utc, Weekday};
use chrono_tz::Tz;

use crate::{error::AppError, time::parse_timezone};

use super::types::{
    CurrentState, RandomEvents, ResolvedState, Schedule, ScheduleSlot, SlotKind, WeeklyTemplate,
};

pub fn resolve(
    schedule: &Schedule,
    character_id: &str,
    now: DateTime<Utc>,
    persisted_random_event: Option<&str>,
    persisted_random_event_date: Option<NaiveDate>,
) -> Result<ResolvedState, AppError> {
    let tz = parse_timezone(&schedule.timezone)?;
    let local = now.with_timezone(&tz);
    let local_date = local.date_naive();
    let local_time = local.time();

    let weekday_key = weekday_template_key(local.weekday());
    let slots = day_slots(&schedule.weekly_template, weekday_key)?;
    let slot = find_matching_slot(slots, local_time)
        .ok_or_else(|| AppError::internal(std::io::Error::other("no matching schedule slot")))?;

    let (random_event, random_event_date) = resolve_random_event(
        &schedule.random_events,
        character_id,
        local_date,
        persisted_random_event,
        persisted_random_event_date,
    );

    let activity_ends_at = slot_end_utc(local_date, local_time, slot, &tz);

    Ok(ResolvedState {
        current: CurrentState {
            weekday_zh: weekday_zh(local.weekday()).to_owned(),
            time_hm: local_time.format("%H:%M").to_string(),
            activity: slot.activity.clone(),
            mood: slot.mood.clone(),
            availability: slot.availability,
            random_event,
        },
        random_event_date,
        activity_ends_at: Some(activity_ends_at),
    })
}

fn resolve_random_event(
    events: &RandomEvents,
    character_id: &str,
    local_date: NaiveDate,
    persisted_random_event: Option<&str>,
    persisted_random_event_date: Option<NaiveDate>,
) -> (Option<String>, NaiveDate) {
    if persisted_random_event_date == Some(local_date) {
        return (persisted_random_event.map(str::to_owned), local_date);
    }

    let event = roll_daily_random_event(events, character_id, local_date);
    (event, local_date)
}

fn roll_daily_random_event(
    events: &RandomEvents,
    character_id: &str,
    date: NaiveDate,
) -> Option<String> {
    if events.pool.is_empty() {
        return None;
    }

    let roll = deterministic_fraction(character_id, date, "random_event");
    if roll >= events.probability {
        return None;
    }

    let idx = (deterministic_fraction(character_id, date, "random_event_pick")
        * events.pool.len() as f64) as usize;
    Some(events.pool[idx.min(events.pool.len() - 1)].clone())
}

fn deterministic_fraction(character_id: &str, date: NaiveDate, salt: &str) -> f64 {
    let mut hasher = DefaultHasher::new();
    character_id.hash(&mut hasher);
    date.hash(&mut hasher);
    salt.hash(&mut hasher);
    (hasher.finish() % 10_000) as f64 / 10_000.0
}

/// 确定性 `[0, 1)`，供主动消息等跨模块使用。
pub(crate) fn deterministic_unit(character_id: &str, date: NaiveDate, salt: &str) -> f64 {
    deterministic_fraction(character_id, date, salt)
}

pub(crate) fn day_slots<'a>(
    template: &'a WeeklyTemplate,
    weekday_key: &str,
) -> Result<&'a [ScheduleSlot], AppError> {
    template
        .slots_for(weekday_key)
        .filter(|slots| !slots.is_empty())
        .ok_or_else(|| AppError::bad_request(format!("schedule_json 缺少 {weekday_key} 模板")))
}

pub fn time_in_slot(time: NaiveTime, start: NaiveTime, end: NaiveTime) -> bool {
    if start <= end {
        time >= start && time < end
    } else {
        time >= start || time < end
    }
}

pub(crate) fn find_matching_slot(
    slots: &[ScheduleSlot],
    time: NaiveTime,
) -> Option<&ScheduleSlot> {
    slots
        .iter()
        .find(|slot| time_in_slot(time, slot.start, slot.end))
}

fn slot_end_utc(
    local_date: NaiveDate,
    local_time: NaiveTime,
    slot: &ScheduleSlot,
    tz: &Tz,
) -> DateTime<Utc> {
    let end_date = if slot.start <= slot.end {
        local_date
    } else if local_time >= slot.start {
        local_date + chrono::Duration::days(1)
    } else {
        local_date
    };
    tz.from_local_datetime(&end_date.and_time(slot.end))
        .single()
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(Utc::now)
}

pub(crate) fn weekday_template_key(weekday: Weekday) -> &'static str {
    match weekday {
        Weekday::Mon => "monday",
        Weekday::Tue => "tuesday",
        Weekday::Wed => "wednesday",
        Weekday::Thu => "thursday",
        Weekday::Fri => "friday",
        Weekday::Sat => "saturday",
        Weekday::Sun => "sunday",
    }
}

pub fn weekday_zh(weekday: Weekday) -> &'static str {
    match weekday {
        Weekday::Mon => "周一",
        Weekday::Tue => "周二",
        Weekday::Wed => "周三",
        Weekday::Thu => "周四",
        Weekday::Fri => "周五",
        Weekday::Sat => "周六",
        Weekday::Sun => "周日",
    }
}

/// 进入活动段已过分钟数（跨午夜时按越过 start 累计）。
pub(crate) fn minutes_since_slot_start(now: NaiveTime, start: NaiveTime) -> u32 {
    let now_m = i64::from(now.num_seconds_from_midnight()) / 60;
    let start_m = i64::from(start.num_seconds_from_midnight()) / 60;
    let mut delta = now_m - start_m;
    if delta < 0 {
        delta += 24 * 60;
    }
    delta as u32
}

/// 每日问候触发窗长度（分钟），落在 `[window_min, window_max]`。
pub(crate) fn daily_greeting_window_mins(
    character_id: &str,
    local_date: NaiveDate,
    window_min: u32,
    window_max: u32,
) -> u32 {
    let min = window_min.min(window_max);
    let max = window_min.max(window_max);
    if min == max {
        return min;
    }
    let unit = deterministic_unit(character_id, local_date, "daily_greeting_window");
    min + ((unit * f64::from(max - min + 1)) as u32).min(max - min)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WakeupSlotStatus {
    pub activity: String,
    pub minutes_into_slot: u32,
    pub local_date: NaiveDate,
}

/// 若 `now` 落在某一活动段内则返回该段的 `kind`。
pub fn current_slot_kind(
    schedule: &Schedule,
    now: DateTime<Utc>,
) -> Result<Option<SlotKind>, AppError> {
    let tz = parse_timezone(&schedule.timezone)?;
    let local = now.with_timezone(&tz);
    let local_time = local.time();
    let weekday_key = weekday_template_key(local.weekday());
    let slots = day_slots(&schedule.weekly_template, weekday_key)?;
    Ok(find_matching_slot(slots, local_time).map(|slot| slot.kind))
}

/// 若 `now` 落在 `kind = wake` 活动段内则返回状态。
pub fn current_wakeup_slot(
    schedule: &Schedule,
    now: DateTime<Utc>,
) -> Result<Option<WakeupSlotStatus>, AppError> {
    let tz = parse_timezone(&schedule.timezone)?;
    let local = now.with_timezone(&tz);
    let local_date = local.date_naive();
    let local_time = local.time();

    let weekday_key = weekday_template_key(local.weekday());
    let slots = day_slots(&schedule.weekly_template, weekday_key)?;
    let Some(current) = find_matching_slot(slots, local_time) else {
        return Ok(None);
    };
    if current.kind != SlotKind::Wake {
        return Ok(None);
    }

    Ok(Some(WakeupSlotStatus {
        activity: current.activity.clone(),
        minutes_into_slot: minutes_since_slot_start(local_time, current.start),
        local_date,
    }))
}

/// 是否在起床段开头的随机问候窗内：`minutes_into_slot < window_mins`。
pub fn in_daily_greeting_window(
    status: &WakeupSlotStatus,
    character_id: &str,
    window_min: u32,
    window_max: u32,
) -> bool {
    let window = daily_greeting_window_mins(character_id, status.local_date, window_min, window_max);
    status.minutes_into_slot < window
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreSleepStatus {
    pub minutes_until_sleep: u32,
    /// 即将开始的 sleep 段所属本地日（用于每日一次统计）。
    pub sleep_local_date: NaiveDate,
}

/// 睡前窗长度（分钟），落在 `[window_min, window_max]`。
pub(crate) fn pre_sleep_window_mins(
    character_id: &str,
    local_date: NaiveDate,
    window_min: u32,
    window_max: u32,
) -> u32 {
    let min = window_min.min(window_max);
    let max = window_min.max(window_max);
    if min == max {
        return min;
    }
    let unit = deterministic_unit(character_id, local_date, "pre_sleep_window");
    min + ((unit * f64::from(max - min + 1)) as u32).min(max - min)
}

/// 若即将切入 `kind=sleep`（且当前不在 sleep 内）则返回距 sleep 开始的分钟数。
pub fn upcoming_pre_sleep(
    schedule: &Schedule,
    now: DateTime<Utc>,
) -> Result<Option<PreSleepStatus>, AppError> {
    let tz = parse_timezone(&schedule.timezone)?;
    let local = now.with_timezone(&tz);
    let local_date = local.date_naive();
    let local_time = local.time();

    let today_key = weekday_template_key(local.weekday());
    let today_slots = day_slots(&schedule.weekly_template, today_key)?;
    if let Some(current) = find_matching_slot(today_slots, local_time) {
        if current.kind == SlotKind::Sleep {
            return Ok(None);
        }
    }

    let Some((sleep_start_local, sleep_date)) =
        next_sleep_start(schedule, &tz, local_date, local_time)?
    else {
        return Ok(None);
    };

    let minutes = (sleep_start_local - local)
        .num_minutes()
        .max(0) as u32;

    Ok(Some(PreSleepStatus {
        minutes_until_sleep: minutes,
        sleep_local_date: sleep_date,
    }))
}

/// 是否落在 sleep 开始前的随机短窗内。
pub fn in_pre_sleep_window(
    status: &PreSleepStatus,
    character_id: &str,
    window_min: u32,
    window_max: u32,
) -> bool {
    let window = pre_sleep_window_mins(
        character_id,
        status.sleep_local_date,
        window_min,
        window_max,
    );
    status.minutes_until_sleep < window
}

/// 在今日与明日模板中找下一场尚未开始的 sleep。
fn next_sleep_start(
    schedule: &Schedule,
    tz: &Tz,
    local_date: NaiveDate,
    local_time: NaiveTime,
) -> Result<Option<(chrono::DateTime<Tz>, NaiveDate)>, AppError> {
    for day_offset in 0i64..=1 {
        let date = local_date + chrono::Duration::days(day_offset);
        let weekday = date.weekday();
        let key = weekday_template_key(weekday);
        let slots = day_slots(&schedule.weekly_template, key)?;
        for slot in slots {
            if slot.kind != SlotKind::Sleep {
                continue;
            }
            let start_local = tz
                .from_local_datetime(&date.and_time(slot.start))
                .single()
                .ok_or_else(|| AppError::bad_request("无效的 sleep 开始时间"))?;
            // 今日：仅取 start > now；明日：整场可用。
            if day_offset == 0 && slot.start <= local_time {
                continue;
            }
            return Ok(Some((start_local, date)));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use serde_json::json;

    fn sample_schedule() -> Schedule {
        Schedule::from_json_value(json!({
            "timezone": "Asia/Shanghai",
            "weekly_template": {
                "monday": [
                    {
                        "start": "07:00",
                        "end": "09:00",
                        "activity": "早餐",
                        "availability": "medium",
                        "mood": "元气",
                        "kind": "wake"
                    },
                    {
                        "start": "09:00",
                        "end": "22:30",
                        "activity": "工作",
                        "availability": "low",
                        "mood": "专注",
                        "kind": "custom"
                    },
                    {
                        "start": "22:30",
                        "end": "07:00",
                        "activity": "睡觉",
                        "availability": "low",
                        "mood": "困倦",
                        "kind": "sleep"
                    }
                ]
            },
            "random_events": {
                "probability": 1.0,
                "pool": ["今日变故"]
            }
        }))
        .unwrap()
    }

    #[test]
    fn time_in_slot_handles_same_day_and_overnight() {
        let nine = NaiveTime::from_hms_opt(9, 0, 0).unwrap();
        let seven = NaiveTime::from_hms_opt(7, 0, 0).unwrap();
        let twenty_three = NaiveTime::from_hms_opt(23, 0, 0).unwrap();
        let six = NaiveTime::from_hms_opt(6, 30, 0).unwrap();

        assert!(time_in_slot(nine, seven, NaiveTime::from_hms_opt(12, 0, 0).unwrap()));
        assert!(time_in_slot(twenty_three, NaiveTime::from_hms_opt(22, 30, 0).unwrap(), seven));
        assert!(time_in_slot(six, NaiveTime::from_hms_opt(22, 30, 0).unwrap(), seven));
        assert!(!time_in_slot(
            NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
            seven,
            NaiveTime::from_hms_opt(9, 0, 0).unwrap()
        ));
    }

    #[test]
    fn resolve_picks_matching_activity() {
        let schedule = sample_schedule();
        // 2026-07-13 is Monday; 10:00 Asia/Shanghai = 02:00 UTC
        let now = Utc.with_ymd_and_hms(2026, 7, 13, 2, 0, 0).unwrap();
        let resolved = resolve(&schedule, "char-1", now, None, None).unwrap();

        assert_eq!(resolved.current.activity, "工作");
        assert_eq!(resolved.current.weekday_zh, "周一");
        assert_eq!(resolved.current.time_hm, "10:00");
        assert_eq!(resolved.current.random_event.as_deref(), Some("今日变故"));
    }

    #[test]
    fn reuses_persisted_random_event_on_same_day() {
        let schedule = sample_schedule();
        let now = Utc.with_ymd_and_hms(2026, 7, 13, 2, 0, 0).unwrap();
        let date = NaiveDate::from_ymd_opt(2026, 7, 13).unwrap();
        let resolved = resolve(&schedule, "char-1", now, Some("已存变故"), Some(date)).unwrap();
        assert_eq!(resolved.current.random_event.as_deref(), Some("已存变故"));
    }

    #[test]
    fn wakeup_slot_uses_kind_wake() {
        let schedule = sample_schedule();
        // 2026-07-13 Monday 07:20 Asia/Shanghai = 2026-07-12 23:20 UTC
        let now = Utc.with_ymd_and_hms(2026, 7, 12, 23, 20, 0).unwrap();
        let status = current_wakeup_slot(&schedule, now).unwrap().unwrap();
        assert_eq!(status.activity, "早餐");
        assert_eq!(status.minutes_into_slot, 20);
        assert!(in_daily_greeting_window(&status, "char-1", 30, 60));
    }

    #[test]
    fn not_wakeup_when_kind_is_custom() {
        let schedule = sample_schedule();
        // 10:00 Asia/Shanghai = 02:00 UTC
        let now = Utc.with_ymd_and_hms(2026, 7, 13, 2, 0, 0).unwrap();
        assert!(current_wakeup_slot(&schedule, now).unwrap().is_none());
    }

    #[test]
    fn greeting_window_is_deterministic_in_range() {
        let date = NaiveDate::from_ymd_opt(2026, 7, 13).unwrap();
        for _ in 0..20 {
            let mins = daily_greeting_window_mins("char-x", date, 30, 60);
            assert!((30..=60).contains(&mins));
        }
        assert_eq!(
            daily_greeting_window_mins("char-x", date, 30, 60),
            daily_greeting_window_mins("char-x", date, 30, 60)
        );
    }

    #[test]
    fn pre_sleep_detects_minutes_until_sleep() {
        let schedule = sample_schedule();
        // 2026-07-13 Monday 22:20 Asia/Shanghai = 14:20 UTC → 10 min until 22:30 sleep
        let now = Utc.with_ymd_and_hms(2026, 7, 13, 14, 20, 0).unwrap();
        let status = upcoming_pre_sleep(&schedule, now).unwrap().unwrap();
        assert_eq!(status.minutes_until_sleep, 10);
        assert_eq!(
            status.sleep_local_date,
            NaiveDate::from_ymd_opt(2026, 7, 13).unwrap()
        );
        assert!(in_pre_sleep_window(&status, "char-1", 10, 30));
    }

    #[test]
    fn pre_sleep_none_while_already_asleep() {
        let schedule = sample_schedule();
        // 23:00 Asia/Shanghai = 15:00 UTC Monday → in overnight sleep
        let now = Utc.with_ymd_and_hms(2026, 7, 13, 15, 0, 0).unwrap();
        assert!(upcoming_pre_sleep(&schedule, now).unwrap().is_none());
    }
}
