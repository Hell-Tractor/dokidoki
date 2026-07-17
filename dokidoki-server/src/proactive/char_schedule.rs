//! 角色日程加载与本地日界线（同 tick 复用一次解析）。

use chrono::{TimeZone, Utc};
use sqlx::MySqlPool;

use crate::{
    db::queries::characters,
    error::AppError,
    schedule::{self, Schedule},
    time::parse_timezone,
};

/// 加载并解析角色日程；无数据或空模板返回 `None`，非法 JSON 打 warn 后返回 `None`。
pub async fn load_schedule(
    pool: &MySqlPool,
    character_id: &str,
    conversation_id: &str,
) -> Result<Option<Schedule>, AppError> {
    let Some(raw) = characters::find_schedule_json(pool, character_id).await? else {
        return Ok(None);
    };
    match Schedule::try_from_json_value(raw) {
        Ok(schedule) => Ok(schedule),
        Err(err) => {
            tracing::warn!(
                conversation_id,
                character_id,
                "invalid schedule_json: {err}"
            );
            Ok(None)
        }
    }
}

pub fn is_in_sleep_slot(
    schedule: Option<&Schedule>,
    now: chrono::DateTime<Utc>,
) -> Result<bool, AppError> {
    let Some(schedule) = schedule else {
        return Ok(false);
    };
    Ok(schedule::current_slot_kind(schedule, now)? == Some(schedule::SlotKind::Sleep))
}

pub fn character_local_day_bounds(
    tz: &str,
    local_date: chrono::NaiveDate,
) -> Result<(chrono::DateTime<Utc>, chrono::DateTime<Utc>), AppError> {
    let tz = parse_timezone(tz)?;
    let start = local_date
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| AppError::bad_request("无效的本地日界线"))?;
    let end = start + chrono::Duration::days(1);
    Ok((
        tz.from_local_datetime(&start)
            .single()
            .ok_or_else(|| AppError::bad_request("无效的本地日界线"))?
            .with_timezone(&Utc),
        tz.from_local_datetime(&end)
            .single()
            .ok_or_else(|| AppError::bad_request("无效的本地日界线"))?
            .with_timezone(&Utc),
    ))
}
