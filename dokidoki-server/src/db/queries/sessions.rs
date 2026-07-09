use sqlx::{Executor, MySql};

use crate::error::AppError;

pub async fn insert<'e, E>(
    executor: E,
    id: &str,
    user_id: &str,
    token_hash: &str,
) -> Result<(), AppError>
where
    E: Executor<'e, Database = MySql>,
{
    sqlx::query(
        r#"
        INSERT INTO user_sessions (id, user_id, token_hash, expires_at)
        VALUES (?, ?, ?, NULL)
        "#,
    )
    .bind(id)
    .bind(user_id)
    .bind(token_hash)
    .execute(executor)
    .await
    .map_err(AppError::from_db)?;

    Ok(())
}
