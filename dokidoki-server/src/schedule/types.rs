use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::Deserialize;

use crate::domain::Availability;
use crate::error::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentState {
    pub weekday_zh: String,
    pub time_hm: String,
    pub activity: String,
    pub mood: String,
    pub availability: Availability,
    pub random_event: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResolvedState {
    pub current: CurrentState,
    pub random_event_date: NaiveDate,
    pub activity_ends_at: Option<DateTime<Utc>>,
}

/// 日程时段语义类型（`schedule_json` 槽位 `kind`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlotKind {
    /// 起床 / 晨间问候段（daily_greeting）
    Wake,
    /// 睡眠段
    Sleep,
    /// 普通自定义活动
    #[default]
    Custom,
}

impl SlotKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Wake => "wake",
            Self::Sleep => "sleep",
            Self::Custom => "custom",
        }
    }
}

/// `characters.schedule_json` 类型化表示。
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Schedule {
    pub timezone: String,
    pub weekly_template: WeeklyTemplate,
    #[serde(default)]
    pub random_events: RandomEvents,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct WeeklyTemplate {
    #[serde(default)]
    pub monday: Vec<ScheduleSlot>,
    #[serde(default)]
    pub tuesday: Vec<ScheduleSlot>,
    #[serde(default)]
    pub wednesday: Vec<ScheduleSlot>,
    #[serde(default)]
    pub thursday: Vec<ScheduleSlot>,
    #[serde(default)]
    pub friday: Vec<ScheduleSlot>,
    #[serde(default)]
    pub saturday: Vec<ScheduleSlot>,
    #[serde(default)]
    pub sunday: Vec<ScheduleSlot>,
}

impl WeeklyTemplate {
    pub fn slots_for(&self, weekday_key: &str) -> Option<&[ScheduleSlot]> {
        let slots = match weekday_key {
            "monday" => &self.monday,
            "tuesday" => &self.tuesday,
            "wednesday" => &self.wednesday,
            "thursday" => &self.thursday,
            "friday" => &self.friday,
            "saturday" => &self.saturday,
            "sunday" => &self.sunday,
            _ => return None,
        };
        Some(slots.as_slice())
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ScheduleSlot {
    #[serde(deserialize_with = "deserialize_hh_mm")]
    pub start: NaiveTime,
    #[serde(deserialize_with = "deserialize_hh_mm")]
    pub end: NaiveTime,
    pub activity: String,
    #[serde(default)]
    pub availability: Availability,
    #[serde(default)]
    pub mood: String,
    #[serde(default)]
    pub kind: SlotKind,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct RandomEvents {
    #[serde(default = "default_event_probability")]
    pub probability: f64,
    #[serde(default)]
    pub pool: Vec<String>,
}

impl Default for RandomEvents {
    fn default() -> Self {
        Self {
            probability: default_event_probability(),
            pool: Vec::new(),
        }
    }
}

fn default_event_probability() -> f64 {
    0.15
}

fn deserialize_hh_mm<'de, D>(deserializer: D) -> Result<NaiveTime, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    NaiveTime::parse_from_str(&value, "%H:%M")
        .map_err(|_| serde::de::Error::custom(format!("无效的 schedule 时间: {value}")))
}

impl Schedule {
    pub fn from_json_value(value: serde_json::Value) -> Result<Self, AppError> {
        if value.is_null() || value.as_object().is_some_and(|o| o.is_empty()) {
            return Err(AppError::bad_request("schedule_json 为空"));
        }
        serde_json::from_value(value).map_err(|err| {
            AppError::bad_request(format!("无效的 schedule_json: {err}"))
        })
    }

    /// 空 JSON → `Ok(None)`；非法结构 → `Err`（由调用方打日志）。
    pub fn try_from_json_value(value: serde_json::Value) -> Result<Option<Self>, AppError> {
        if value.is_null() || value.as_object().is_some_and(|o| o.is_empty()) {
            return Ok(None);
        }
        Self::from_json_value(value).map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn deserializes_slot_kind() {
        let schedule = Schedule::from_json_value(json!({
            "timezone": "Asia/Shanghai",
            "weekly_template": {
                "monday": [
                    {
                        "start": "07:00",
                        "end": "09:00",
                        "activity": "做早餐",
                        "availability": "medium",
                        "mood": "元气",
                        "kind": "wake"
                    },
                    {
                        "start": "22:30",
                        "end": "07:00",
                        "activity": "睡觉",
                        "kind": "sleep"
                    }
                ]
            }
        }))
        .unwrap();

        assert_eq!(schedule.weekly_template.monday[0].kind, SlotKind::Wake);
        assert_eq!(schedule.weekly_template.monday[1].kind, SlotKind::Sleep);
        assert_eq!(
            schedule.weekly_template.monday[0].start,
            NaiveTime::from_hms_opt(7, 0, 0).unwrap()
        );
    }

    #[test]
    fn missing_kind_defaults_to_custom() {
        let schedule = Schedule::from_json_value(json!({
            "timezone": "UTC",
            "weekly_template": {
                "monday": [
                    {"start": "09:00", "end": "18:00", "activity": "工作", "availability": "low"}
                ]
            }
        }))
        .unwrap();
        assert_eq!(schedule.weekly_template.monday[0].kind, SlotKind::Custom);
    }
}
