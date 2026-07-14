pub mod resolver;
pub mod scheduler;
pub mod types;

use chrono::{Datelike, Utc};
use sqlx::MySqlPool;

use crate::{
    db::queries::{character_states as state_queries, characters as character_queries},
    error::AppError,
    time::parse_timezone,
};

pub use resolver::resolve;
pub use resolver::{
    current_custom_slot, current_slot_kind, current_wakeup_slot, in_daily_greeting_window,
    in_pre_sleep_window, in_schedule_change_window, upcoming_pre_sleep, CustomSlotStatus,
    PreSleepStatus, WakeupSlotStatus,
};
pub use scheduler::run as run_scheduler;
pub use types::{CurrentState, Schedule, SlotKind};

/// 按 `schedule_json` 解析当前状态，写回 `character_states` 并返回。
pub async fn refresh_character_state(
    pool: &MySqlPool,
    character_id: &str,
) -> Result<CurrentState, AppError> {
    let raw = character_queries::find_schedule_json(pool, character_id)
        .await?
        .ok_or_else(|| AppError::not_found("角色不存在"))?;

    let schedule = match Schedule::try_from_json_value(raw) {
        Ok(None) => {
            tracing::debug!(
                character_id = %character_id,
                "schedule refresh: empty schedule_json, using default state"
            );
            return Ok(default_state());
        }
        Ok(Some(schedule)) => schedule,
        Err(err) => {
            tracing::warn!(
                character_id = %character_id,
                "schedule refresh skipped: invalid schedule_json: {err}"
            );
            return Err(err);
        }
    };

    let persisted = state_queries::find_by_character_id(pool, character_id).await?;
    let (persisted_event, persisted_event_date) = match &persisted {
        Some(row) => (row.random_event.as_deref(), row.random_event_date),
        None => (None, None),
    };
    let resolved = resolve(
        &schedule,
        character_id,
        Utc::now(),
        persisted_event,
        persisted_event_date,
    )?;

    tracing::debug!(
        character_id = %character_id,
        activity = %resolved.current.activity,
        availability = %resolved.current.availability,
        "schedule state refreshed"
    );

    state_queries::upsert(
        pool,
        character_id,
        state_queries::UpsertStateParams {
            current_activity: &resolved.current.activity,
            current_mood: &resolved.current.mood,
            availability: resolved.current.availability,
            activity_ends_at: resolved.activity_ends_at,
            random_event: resolved.current.random_event.as_deref(),
            random_event_date: Some(resolved.random_event_date),
        },
    )
    .await?;

    Ok(resolved.current)
}

/// 刷新所有配置了 `schedule_json` 的角色状态。
pub async fn refresh_all_character_states(pool: &MySqlPool) -> Result<(), AppError> {
    let ids = character_queries::list_character_ids(pool).await?;
    for id in ids {
        if let Err(err) = refresh_character_state(pool, &id).await {
            tracing::warn!(character_id = %id, "schedule refresh failed: {err}");
        }
    }
    Ok(())
}

/// 从 `character_states` 读取活动状态，并按角色时区计算当前时刻展示字段。
/// 若尚无状态行（如 scheduler 首次 tick 前），回退一次即时解析。
pub async fn load_current_state_for_prompt(
    pool: &MySqlPool,
    character_id: &str,
) -> Result<Option<CurrentState>, AppError> {
    let raw = character_queries::find_schedule_json(pool, character_id).await?;
    let Some(raw) = raw else {
        tracing::trace!(character_id = %character_id, "prompt schedule: no schedule_json");
        return Ok(None);
    };
    let schedule = match Schedule::try_from_json_value(raw) {
        Ok(None) => {
            tracing::trace!(character_id = %character_id, "prompt schedule: empty schedule_json");
            return Ok(None);
        }
        Ok(Some(schedule)) => schedule,
        Err(err) => {
            tracing::warn!(
                character_id = %character_id,
                "prompt schedule load skipped: invalid schedule_json: {err}"
            );
            return Ok(None);
        }
    };

    let row = state_queries::find_prompt_fields(pool, character_id).await?;
    let Some(row) = row else {
        return Ok(Some(refresh_character_state(pool, character_id).await?));
    };

    let tz = parse_timezone(&schedule.timezone)?;
    let local = Utc::now().with_timezone(&tz);

    Ok(Some(CurrentState {
        weekday_zh: resolver::weekday_zh(local.weekday()).to_owned(),
        time_hm: local.format("%H:%M").to_string(),
        activity: row.current_activity,
        mood: row.current_mood,
        availability: row.availability,
        random_event: row.random_event,
    }))
}

fn default_state() -> CurrentState {
    CurrentState {
        weekday_zh: resolver::weekday_zh(Utc::now().weekday()).to_owned(),
        time_hm: Utc::now().format("%H:%M").to_string(),
        activity: String::new(),
        mood: String::new(),
        availability: crate::domain::Availability::Medium,
        random_event: None,
    }
}
