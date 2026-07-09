use chrono::NaiveDate;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct User {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub birthday: Option<NaiveDate>,
    pub max_proactive_per_day: i32,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserCredentials {
    pub id: String,
    pub username: String,
    pub password_hash: String,
    pub display_name: String,
    pub birthday: Option<NaiveDate>,
    pub max_proactive_per_day: i32,
}

impl From<UserCredentials> for User {
    fn from(value: UserCredentials) -> Self {
        Self {
            id: value.id,
            username: value.username,
            display_name: value.display_name,
            birthday: value.birthday,
            max_proactive_per_day: value.max_proactive_per_day,
        }
    }
}
