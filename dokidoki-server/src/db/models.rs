use chrono::NaiveDate;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct User {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub birthday: Option<NaiveDate>,
    pub timezone: String,
    pub max_proactive_per_day: i32,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserCredentials {
    pub id: String,
    pub username: String,
    pub password_hash: String,
    pub display_name: String,
    pub birthday: Option<NaiveDate>,
    pub timezone: String,
    pub max_proactive_per_day: i32,
}

impl From<UserCredentials> for User {
    fn from(value: UserCredentials) -> Self {
        Self {
            id: value.id,
            username: value.username,
            display_name: value.display_name,
            birthday: value.birthday,
            timezone: value.timezone,
            max_proactive_per_day: value.max_proactive_per_day,
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserCharacterSettings {
    pub dnd_start: Option<chrono::NaiveTime>,
    pub dnd_end: Option<chrono::NaiveTime>,
    pub push_muted: bool,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Character {
    pub id: String,
    pub name: String,
    pub avatar_path: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Conversation {
    pub id: String,
    pub user_id: String,
    pub character_id: String,
    pub status: crate::domain::ConversationStatus,
    pub winding_reason: Option<crate::domain::WindingReason>,
    pub first_contact_done: bool,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ConversationListRow {
    pub id: String,
    pub character_id: String,
    pub character_name: String,
    pub status: crate::domain::ConversationStatus,
    pub current_activity: Option<String>,
    pub last_message_content: Option<String>,
    pub last_message_created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_message_role: Option<String>,
}
