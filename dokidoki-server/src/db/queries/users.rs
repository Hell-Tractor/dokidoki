use chrono::NaiveDate;
use sqlx::{Executor, MySql, MySqlPool};

use crate::{
    db::models::{User, UserCredentials},
    error::AppError,
};

pub async fn find_by_username(
    pool: &MySqlPool,
    username: &str,
) -> Result<Option<UserCredentials>, AppError> {
    let user = sqlx::query_as::<_, UserCredentials>(
        r#"
        SELECT id, username, password_hash, display_name, birthday, max_proactive_per_day
        FROM users
        WHERE username = ?
        "#,
    )
    .bind(username)
    .fetch_optional(pool)
    .await
    .map_err(AppError::from_db)?;

    Ok(user)
}

pub async fn find_by_id(pool: &MySqlPool, id: &str) -> Result<Option<User>, AppError> {
    let user = sqlx::query_as::<_, User>(
        r#"
        SELECT id, username, display_name, birthday, max_proactive_per_day
        FROM users
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::from_db)?;

    Ok(user)
}

pub struct UpdateMeParams {
    pub display_name: Option<String>,
    pub birthday: Option<NaiveDate>,
    pub max_proactive_per_day: Option<i32>,
}

pub async fn update_profile(
    pool: &MySqlPool,
    id: &str,
    current: &User,
    params: UpdateMeParams,
) -> Result<User, AppError> {
    let display_name = params
        .display_name
        .unwrap_or_else(|| current.display_name.clone());
    let birthday = params.birthday.or(current.birthday);
    let max_proactive_per_day = params
        .max_proactive_per_day
        .unwrap_or(current.max_proactive_per_day);

    sqlx::query(
        r#"
        UPDATE users
        SET display_name = ?, birthday = ?, max_proactive_per_day = ?
        WHERE id = ?
        "#,
    )
    .bind(&display_name)
    .bind(birthday)
    .bind(max_proactive_per_day)
    .bind(id)
    .execute(pool)
    .await
    .map_err(AppError::from_db)?;

    Ok(User {
        id: id.to_owned(),
        username: current.username.clone(),
        display_name,
        birthday,
        max_proactive_per_day,
    })
}

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
