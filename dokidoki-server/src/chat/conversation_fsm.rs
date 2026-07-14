//! 会话状态机：active / winding_down / paused

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversationStatus {
    Active,
    WindingDown,
    Paused,
}

impl ConversationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::WindingDown => "winding_down",
            Self::Paused => "paused",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "active" => Some(Self::Active),
            "winding_down" => Some(Self::WindingDown),
            "paused" => Some(Self::Paused),
            _ => None,
        }
    }
}

/// 用户发消息后的处理决策。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserMessageDecision {
    /// 继续调用 LLM；`status` 有值时先更新会话状态。
    CallLlm { status: Option<ConversationStatus> },
    /// 进入 paused，不调用 LLM。
    PauseWithoutReply,
    /// 保持 paused，不调用 LLM。
    IgnoreWhilePaused,
}

pub fn on_user_message(
    current_status: ConversationStatus,
    message: &str,
    pause_on_farewell: bool,
) -> UserMessageDecision {
    let substantive = is_substantive(message);
    let farewell = is_farewell(message);

    match current_status {
        ConversationStatus::Active => UserMessageDecision::CallLlm { status: None },
        ConversationStatus::WindingDown => {
            if substantive {
                UserMessageDecision::CallLlm {
                    status: Some(ConversationStatus::Active),
                }
            } else if farewell && pause_on_farewell {
                UserMessageDecision::PauseWithoutReply
            } else {
                UserMessageDecision::CallLlm { status: None }
            }
        }
        ConversationStatus::Paused => {
            if substantive {
                UserMessageDecision::CallLlm {
                    status: Some(ConversationStatus::Active),
                }
            } else {
                UserMessageDecision::IgnoreWhilePaused
            }
        }
    }
}

/// LLM 输出后的状态变更（若有）。
pub fn status_after_llm_action(action: super::parser::LlmAction) -> Option<ConversationStatus> {
    match action {
        super::parser::LlmAction::EndTopic(_) => Some(ConversationStatus::WindingDown),
        super::parser::LlmAction::NoReply | super::parser::LlmAction::Reply(_) => None,
    }
}

pub fn is_farewell(message: &str) -> bool {
    const PHRASES: &[&str] = &[
        "好的", "好哒", "好吧", "拜拜", "拜", "去吧", "嗯嗯", "晚安", "886", "88", "知道了",
        "嗯", "哦", "好", "行", "ok", "OK",
    ];
    let trimmed = message.trim();
    if trimmed.is_empty() {
        return false;
    }
    PHRASES
        .iter()
        .any(|phrase| trimmed.eq_ignore_ascii_case(phrase))
}

pub fn is_substantive(message: &str) -> bool {
    let trimmed = message.trim();
    !trimmed.is_empty() && !is_farewell(trimmed) && trimmed.chars().count() > 2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paused_resumes_on_substantive_message() {
        let decision = on_user_message(ConversationStatus::Paused, "今天好累啊", true);
        assert_eq!(
            decision,
            UserMessageDecision::CallLlm {
                status: Some(ConversationStatus::Active)
            }
        );
    }

    #[test]
    fn paused_ignores_short_farewell() {
        let decision = on_user_message(ConversationStatus::Paused, "嗯", true);
        assert_eq!(decision, UserMessageDecision::IgnoreWhilePaused);
    }

    #[test]
    fn winding_down_farewell_pauses_when_configured() {
        let decision = on_user_message(ConversationStatus::WindingDown, "拜拜", true);
        assert_eq!(decision, UserMessageDecision::PauseWithoutReply);
    }

    #[test]
    fn winding_down_farewell_keeps_chatting_when_pause_disabled() {
        let decision = on_user_message(ConversationStatus::WindingDown, "拜拜", false);
        assert_eq!(decision, UserMessageDecision::CallLlm { status: None });
    }

    #[test]
    fn winding_down_substantive_resumes_active() {
        let decision = on_user_message(ConversationStatus::WindingDown, "等等我还有事", true);
        assert_eq!(
            decision,
            UserMessageDecision::CallLlm {
                status: Some(ConversationStatus::Active)
            }
        );
    }

    #[test]
    fn end_topic_sets_winding_down() {
        let action = super::super::parser::parse_action("[END_TOPIC]我先走了|||等下聊");
        assert_eq!(
            status_after_llm_action(action),
            Some(ConversationStatus::WindingDown)
        );
    }
}
