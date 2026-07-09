use chrono::NaiveDate;
use sqlx::{Executor, MySql};

use crate::{db::models::User, error::AppError};

pub async fn insert<'e, E>(
    executor: E,
    id: &str,
    username: &str,
    password_hash: &str,
    display_name: &str,
    birthday: Option<NaiveDate>,
) -> Result<User, AppError>
where
    E: Executor<'e, Database = MySql>,
{
    sqlx::query(
        r#"
        INSERT INTO users (id, username, password_hash, display_name, birthday)
        VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(id)
    .bind(username)
    .bind(password_hash)
    .bind(display_name)
    .bind(birthday)
    .execute(executor)
    .await
    .map_err(AppError::from_db)?;

    Ok(User {
        id: id.to_owned(),
        username: username.to_owned(),
        display_name: display_name.to_owned(),
        birthday,
        max_proactive_per_day: 20,
    })
}
