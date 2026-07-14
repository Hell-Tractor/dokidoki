//! 主动消息触发器。骨架阶段全部返回 `None`，后续按优先级逐个实现。

/// 触发类型（与 `proactive_logs.trigger_type` / Prompt `{proactive_trigger}` 对齐）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerType {
    DailyGreeting,
    ReEngage,
    SilenceWake,
    MoodFollowup,
    ScheduleChange,
}

impl TriggerType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DailyGreeting => "daily_greeting",
            Self::ReEngage => "re_engage",
            Self::SilenceWake => "silence_wake",
            Self::MoodFollowup => "mood_followup",
            Self::ScheduleChange => "schedule_change",
        }
    }
}

/// 触发器求值上下文（字段随各类触发落地逐步补齐）。
#[derive(Debug, Clone)]
pub struct TriggerContext<'a> {
    pub conversation_id: &'a str,
    pub status: &'a str,
    pub availability: &'a str,
}

/// 按优先级取一条：daily_greeting → re_engage → silence_wake → mood_followup → schedule_change。
pub fn select_trigger(ctx: &TriggerContext<'_>) -> Option<TriggerType> {
    for candidate in [
        TriggerType::DailyGreeting,
        TriggerType::ReEngage,
        TriggerType::SilenceWake,
        TriggerType::MoodFollowup,
        TriggerType::ScheduleChange,
    ] {
        if evaluate(candidate, ctx) {
            return Some(candidate);
        }
    }
    None
}

fn evaluate(trigger: TriggerType, _ctx: &TriggerContext<'_>) -> bool {
    match trigger {
        TriggerType::DailyGreeting => false,
        TriggerType::ReEngage => false,
        TriggerType::SilenceWake => false,
        TriggerType::MoodFollowup => false,
        TriggerType::ScheduleChange => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skeleton_selects_no_trigger() {
        let ctx = TriggerContext {
            conversation_id: "c1",
            status: "active",
            availability: "high",
        };
        assert_eq!(select_trigger(&ctx), None);
    }
}
