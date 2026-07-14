use chrono::Utc;
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::{
    db::queries::memories as memory_queries,
    error::AppError,
};

use super::parser::{ForgetMemoryAction, ParsedLlmResponse, StoreMemoryAction};

pub async fn apply_side_effects(
    pool: &MySqlPool,
    user_id: &str,
    character_id: &str,
    parsed: &ParsedLlmResponse,
) -> Result<(), AppError> {
    for action in &parsed.store_memories {
        apply_store(pool, user_id, character_id, action).await?;
    }
    for action in &parsed.forget_memories {
        apply_forget(pool, user_id, character_id, action).await?;
    }
    Ok(())
}

pub async fn apply_store(
    pool: &MySqlPool,
    user_id: &str,
    character_id: &str,
    action: &StoreMemoryAction,
) -> Result<(), AppError> {
    let now = Utc::now();
    let expires_at = action.memory_type.expires_at(now);
    memory_queries::upsert(
        pool,
        &Uuid::new_v4().to_string(),
        user_id,
        character_id,
        &action.content,
        action.memory_type.as_str(),
        action.memory_key.as_deref(),
        expires_at,
    )
    .await?;
    tracing::info!(
        user_id = %user_id,
        character_id = %character_id,
        memory_type = action.memory_type.as_str(),
        memory_key = ?action.memory_key,
        "memory stored"
    );
    Ok(())
}

pub async fn apply_forget(
    pool: &MySqlPool,
    user_id: &str,
    character_id: &str,
    action: &ForgetMemoryAction,
) -> Result<(), AppError> {
    let deleted = memory_queries::forget_by_key(pool, user_id, character_id, &action.target).await?;
    let deleted = if deleted == 0 {
        memory_queries::forget_by_keyword(pool, user_id, character_id, &action.target).await?
    } else {
        deleted
    };
    if deleted == 0 {
        tracing::debug!(
            user_id = %user_id,
            character_id = %character_id,
            target = %action.target,
            "memory forget matched no rows"
        );
    } else {
        tracing::info!(
            user_id = %user_id,
            character_id = %character_id,
            target = %action.target,
            deleted,
            "memory forgotten"
        );
    }
    Ok(())
}

pub async fn purge_expired(pool: &MySqlPool) -> Result<u64, AppError> {
    memory_queries::delete_expired(pool).await
}
