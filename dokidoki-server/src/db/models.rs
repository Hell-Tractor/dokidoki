use chrono::NaiveDate;

#[derive(Debug, Clone)]
pub struct User {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub birthday: Option<NaiveDate>,
    pub max_proactive_per_day: i32,
}
