use chrono::{DateTime, NaiveDate, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentState {
    pub weekday_zh: String,
    pub time_hm: String,
    pub activity: String,
    pub mood: String,
    pub availability: String,
    pub random_event: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResolvedState {
    pub current: CurrentState,
    pub random_event_date: NaiveDate,
    pub activity_ends_at: Option<DateTime<Utc>>,
}
