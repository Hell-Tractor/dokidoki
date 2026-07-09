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

pub async fn find_user_id_by_token_hash(
    pool: &sqlx::MySqlPool,
    token_hash: &str,
) -> Result<Option<String>, AppError> {
    let user_id = sqlx::query_scalar(
        r#"
        SELECT user_id
        FROM user_sessions
        WHERE token_hash = ?
          AND (expires_at IS NULL OR expires_at > NOW(6))
        "#,
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await
    .map_err(AppError::from_db)?;

    Ok(user_id)
}
