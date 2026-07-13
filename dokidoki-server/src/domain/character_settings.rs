use chrono::NaiveTime;
use sqlx::MySqlPool;

use crate::{
    db::{
        models::UserCharacterSettings,
        queries::{character_settings as settings_queries, characters as character_queries},
    },
    error::AppError,
};

pub struct CharacterSettings {
    pub dnd_start: Option<NaiveTime>,
    pub dnd_end: Option<NaiveTime>,
    pub push_muted: bool,
}

impl Default for CharacterSettings {
    fn default() -> Self {
        Self {
            dnd_start: None,
            dnd_end: None,
            push_muted: false,
        }
    }
}

impl From<UserCharacterSettings> for CharacterSettings {
    fn from(value: UserCharacterSettings) -> Self {
        Self {
            dnd_start: value.dnd_start,
            dnd_end: value.dnd_end,
            push_muted: value.push_muted,
        }
    }
}

pub struct UpdateCharacterSettingsInput {
    pub dnd_start: PatchField<NaiveTime>,
    pub dnd_end: PatchField<NaiveTime>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatchField<T> {
    Unset,
    Clear,
    Set(T),
}

pub fn parse_wall_clock(value: &str) -> Result<NaiveTime, AppError> {
    NaiveTime::parse_from_str(value, "%H:%M")
        .map_err(|_| AppError::bad_request(format!("无效的时间格式: {value}，应为 HH:MM")))
}

async fn ensure_character_exists(pool: &MySqlPool, character_id: &str) -> Result<(), AppError> {
    if character_queries::find_by_id(pool, character_id)
        .await?
        .is_some()
    {
        Ok(())
    } else {
        Err(AppError::not_found("角色不存在"))
    }
}

pub async fn get_for_user(
    pool: &MySqlPool,
    user_id: &str,
    character_id: &str,
) -> Result<CharacterSettings, AppError> {
    ensure_character_exists(pool, character_id).await?;

    Ok(settings_queries::find_by_user_and_character(pool, user_id, character_id)
        .await?
        .map(CharacterSettings::from)
        .unwrap_or_default())
}

pub async fn update_for_user(
    pool: &MySqlPool,
    user_id: &str,
    character_id: &str,
    input: UpdateCharacterSettingsInput,
) -> Result<CharacterSettings, AppError> {
    ensure_character_exists(pool, character_id).await?;

    let current = get_for_user(pool, user_id, character_id).await?;
    let dnd_start = apply_patch(input.dnd_start, current.dnd_start);
    let dnd_end = apply_patch(input.dnd_end, current.dnd_end);

    let updated = settings_queries::upsert(
        pool,
        user_id,
        character_id,
        settings_queries::UpsertSettingsParams {
            dnd_start,
            dnd_end,
        },
    )
    .await?;

    Ok(updated.into())
}

fn apply_patch<T>(patch: PatchField<T>, current: Option<T>) -> Option<T> {
    match patch {
        PatchField::Unset => current,
        PatchField::Clear => None,
        PatchField::Set(value) => Some(value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_wall_clock_accepts_hh_mm() {
        let time = parse_wall_clock("23:30").unwrap();
        assert_eq!(time.format("%H:%M").to_string(), "23:30");
    }

    #[test]
    fn apply_patch_respects_unset_clear_set() {
        assert_eq!(apply_patch(PatchField::Unset, Some(NaiveTime::MIN)), Some(NaiveTime::MIN));
        assert_eq!(apply_patch(PatchField::<NaiveTime>::Clear, Some(NaiveTime::MIN)), None);
        assert_eq!(
            apply_patch(PatchField::Set(NaiveTime::MIN), None),
            Some(NaiveTime::MIN)
        );
    }
}
