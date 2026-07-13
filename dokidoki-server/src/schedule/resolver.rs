use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use chrono::{DateTime, Datelike, NaiveDate, NaiveTime, TimeZone, Utc, Weekday};
use chrono_tz::Tz;
use serde_json::Value;

use crate::{error::AppError, time::parse_timezone};

use super::types::{CurrentState, ResolvedState};

#[derive(Debug, Clone)]
struct TimeSlot {
    start: NaiveTime,
    end: NaiveTime,
    activity: String,
    mood: String,
    availability: String,
}

pub fn resolve(
    schedule: &Value,
    character_id: &str,
    now: DateTime<Utc>,
    persisted_random_event: Option<&str>,
    persisted_random_event_date: Option<NaiveDate>,
) -> Result<ResolvedState, AppError> {
    let tz_str = schedule
        .get("timezone")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::bad_request("schedule_json 缺少 timezone"))?;
    let tz = parse_timezone(tz_str)?;
    let local = now.with_timezone(&tz);
    let local_date = local.date_naive();
    let local_time = local.time();

    let weekday_key = weekday_template_key(local.weekday());
    let slots = parse_day_slots(schedule, weekday_key)?;
    let slot = find_matching_slot(&slots, local_time)
        .ok_or_else(|| AppError::internal(std::io::Error::other("no matching schedule slot")))?;

    let (random_event, random_event_date) = resolve_random_event(
        schedule,
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
            availability: slot.availability.clone(),
            random_event,
        },
        random_event_date,
        activity_ends_at: Some(activity_ends_at),
    })
}

fn resolve_random_event(
    schedule: &Value,
    character_id: &str,
    local_date: NaiveDate,
    persisted_random_event: Option<&str>,
    persisted_random_event_date: Option<NaiveDate>,
) -> (Option<String>, NaiveDate) {
    if persisted_random_event_date == Some(local_date) {
        return (persisted_random_event.map(str::to_owned), local_date);
    }

    let event = roll_daily_random_event(schedule, character_id, local_date);
    (event, local_date)
}

fn roll_daily_random_event(
    schedule: &Value,
    character_id: &str,
    date: NaiveDate,
) -> Option<String> {
    let events = schedule.get("random_events")?;
    let probability = events.get("probability").and_then(Value::as_f64).unwrap_or(0.15);
    let pool = events.get("pool").and_then(Value::as_array)?;
    let items: Vec<&str> = pool.iter().filter_map(Value::as_str).collect();
    if items.is_empty() {
        return None;
    }

    let roll = deterministic_fraction(character_id, date, "random_event");
    if roll >= probability {
        return None;
    }

    let idx = (deterministic_fraction(character_id, date, "random_event_pick")
        * items.len() as f64) as usize;
    Some(items[idx.min(items.len() - 1)].to_owned())
}

fn deterministic_fraction(character_id: &str, date: NaiveDate, salt: &str) -> f64 {
    let mut hasher = DefaultHasher::new();
    character_id.hash(&mut hasher);
    date.hash(&mut hasher);
    salt.hash(&mut hasher);
    (hasher.finish() % 10_000) as f64 / 10_000.0
}

fn parse_day_slots(schedule: &Value, weekday_key: &str) -> Result<Vec<TimeSlot>, AppError> {
    let template = schedule
        .get("weekly_template")
        .and_then(|v| v.get(weekday_key))
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::bad_request(format!("schedule_json 缺少 {weekday_key} 模板")))?;

    template
        .iter()
        .map(parse_slot)
        .collect()
}

fn parse_slot(value: &Value) -> Result<TimeSlot, AppError> {
    let start = parse_hh_mm(value.get("start").and_then(Value::as_str).unwrap_or(""))?;
    let end = parse_hh_mm(value.get("end").and_then(Value::as_str).unwrap_or(""))?;
    let activity = value
        .get("activity")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    let mood = value
        .get("mood")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    let availability = value
        .get("availability")
        .and_then(Value::as_str)
        .unwrap_or("medium")
        .to_owned();

    Ok(TimeSlot {
        start,
        end,
        activity,
        mood,
        availability,
    })
}

fn parse_hh_mm(value: &str) -> Result<NaiveTime, AppError> {
    NaiveTime::parse_from_str(value, "%H:%M")
        .map_err(|_| AppError::bad_request(format!("无效的 schedule 时间: {value}")))
}

pub fn time_in_slot(time: NaiveTime, start: NaiveTime, end: NaiveTime) -> bool {
    if start <= end {
        time >= start && time < end
    } else {
        time >= start || time < end
    }
}

fn find_matching_slot(slots: &[TimeSlot], time: NaiveTime) -> Option<&TimeSlot> {
    slots.iter().find(|slot| time_in_slot(time, slot.start, slot.end))
}

fn slot_end_utc(
    local_date: NaiveDate,
    local_time: NaiveTime,
    slot: &TimeSlot,
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

fn weekday_template_key(weekday: Weekday) -> &'static str {
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use serde_json::json;

    fn sample_schedule() -> Value {
        json!({
            "timezone": "Asia/Shanghai",
            "weekly_template": {
                "monday": [
                    {"start": "07:00", "end": "09:00", "activity": "早餐", "availability": "medium", "mood": "元气"},
                    {"start": "09:00", "end": "22:30", "activity": "工作", "availability": "low", "mood": "专注"},
                    {"start": "22:30", "end": "07:00", "activity": "睡觉", "availability": "low", "mood": "困倦"}
                ]
            },
            "random_events": {
                "probability": 1.0,
                "pool": ["今日变故"]
            }
        })
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
}
