use chrono::{DateTime, Utc};

use crate::{
    config::Summary,
    db::{
        message::Message,
        queries::{conversations as conversation_queries, messages as message_queries},
    },
    error::AppError,
    llm::LlmClient,
};

use crate::prompt::build_summary_request;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnInfo {
    pub turn_id: String,
    pub started_at: DateTime<Utc>,
}

pub fn should_compact(total_turns: usize, trigger_turns: u32) -> bool {
    total_turns > trigger_turns as usize
}

pub fn select_turns_to_compact(
    turns: &[TurnInfo],
    keep_recent_turns: u32,
    covers_until: Option<DateTime<Utc>>,
) -> Vec<String> {
    let keep = keep_recent_turns as usize;
    if turns.len() <= keep {
        return Vec::new();
    }

    let compact_end = turns.len() - keep;
    turns
        .iter()
        .take(compact_end)
        .filter(|turn| covers_until.is_none_or(|until| turn.started_at > until))
        .map(|turn| turn.turn_id.clone())
        .collect()
}

pub fn format_messages_for_summary(messages: &[Message]) -> String {
    messages
        .iter()
        .filter_map(|message| {
            let content = message.content.as_ref()?.trim();
            if content.is_empty() {
                return None;
            }
            let speaker = match message.role.as_str() {
                "user" => "用户",
                "character" => "角色",
                _ => return None,
            };
            Some(format!("{speaker}: {content}"))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn truncate_summary(summary: &str, max_chars: u32) -> String {
    let max = max_chars as usize;
    if summary.chars().count() <= max {
        return summary.to_owned();
    }
    summary.chars().take(max).collect()
}

pub async fn run_compact(
    db: &sqlx::MySqlPool,
    llm: &LlmClient,
    conversation_id: &str,
    config: &Summary,
) -> Result<(), AppError> {
    let summary_fields = conversation_queries::find_summary_fields(db, conversation_id)
        .await?
        .ok_or_else(|| AppError::not_found("会话不存在"))?;

    let turns = message_queries::list_turns(db, conversation_id)
        .await?
        .into_iter()
        .map(|row| TurnInfo {
            turn_id: row.turn_id,
            started_at: row.started_at,
        })
        .collect::<Vec<_>>();
    if !should_compact(turns.len(), config.trigger_turns) {
        return Ok(());
    }

    let turn_ids = select_turns_to_compact(
        &turns,
        config.keep_recent_turns,
        summary_fields.summary_covers_until,
    );
    if turn_ids.is_empty() {
        tracing::debug!(
            conversation_id = %conversation_id,
            total_turns = turns.len(),
            "summary compact skipped: no new turns past covers_until"
        );
        return Ok(());
    }

    let messages =
        message_queries::list_text_messages_for_turn_ids(db, conversation_id, &turn_ids).await?;
    let messages_text = format_messages_for_summary(&messages);
    if messages_text.is_empty() {
        tracing::warn!(
            conversation_id = %conversation_id,
            turns = turn_ids.len(),
            "summary compact skipped: selected turns have no text"
        );
        return Ok(());
    }

    tracing::info!(
        conversation_id = %conversation_id,
        turns = turn_ids.len(),
        "summary compact starting"
    );

    let request = build_summary_request(
        conversation_id,
        summary_fields.summary.as_deref(),
        &messages_text,
        config.max_summary_chars,
    );
    let raw = llm.chat(request).await?;
    let merged = truncate_summary(raw.trim(), config.max_summary_chars);
    if merged.is_empty() {
        tracing::warn!(
            conversation_id = %conversation_id,
            "summary compact skipped: empty llm summary"
        );
        return Ok(());
    }

    let covers_until = messages
        .iter()
        .map(|message| message.created_at)
        .max()
        .or_else(|| {
            turns
                .iter()
                .filter(|turn| turn_ids.contains(&turn.turn_id))
                .map(|turn| turn.started_at)
                .max()
        })
        .ok_or_else(|| AppError::internal(std::io::Error::other("compact covers_until missing")))?;

    conversation_queries::update_summary(db, conversation_id, &merged, covers_until).await?;
    tracing::info!(
        conversation_id = %conversation_id,
        summary_chars = merged.chars().count(),
        "summary compact saved"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn turn(id: &str, minute: u32) -> TurnInfo {
        TurnInfo {
            turn_id: id.into(),
            started_at: Utc.timestamp_opt(1_700_000_000 + i64::from(minute), 0).unwrap(),
        }
    }

    #[test]
    fn should_compact_when_turns_exceed_threshold() {
        assert!(!should_compact(80, 80));
        assert!(should_compact(81, 80));
    }

    #[test]
    fn selects_only_uncovered_turns_outside_recent_window() {
        let turns: Vec<_> = (0..81).map(|i| turn(&format!("t{i}"), i)).collect();
        let covers_until = turn("t39", 39).started_at;
        let selected = select_turns_to_compact(&turns, 40, Some(covers_until));
        assert_eq!(selected, vec!["t40".to_string()]);
    }

    #[test]
    fn first_compact_takes_all_non_recent_turns() {
        let turns: Vec<_> = (0..81).map(|i| turn(&format!("t{i}"), i)).collect();
        let selected = select_turns_to_compact(&turns, 40, None);
        assert_eq!(selected.len(), 41);
        assert_eq!(selected.first().unwrap(), "t0");
        assert_eq!(selected.last().unwrap(), "t40");
    }
}
