//! 会话状态机：active / winding_down / paused* 。

use crate::domain::Availability;

pub use crate::domain::{ConversationStatus, WindingReason};

/// 用户发消息后的处理决策。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserMessageDecision {
    /// 继续调用 LLM；`status` 有值时先更新会话状态。
    CallLlm { status: Option<ConversationStatus> },
    /// 按 `winding_reason` 进入终态暂停，不调用 LLM。
    PauseWithoutReply,
    /// 保持终态暂停，不调用 LLM。
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
        ConversationStatus::Paused
        | ConversationStatus::PausedCharBusy
        | ConversationStatus::PausedUserBusy => {
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

/// 由 LLM 输出推导 `winding_down` 原因；`None` 表示不改变状态。
///
/// 规则：`[USER_BUSY]` 优先；否则 `[END_TOPIC]` + availability=low → char_busy；
/// 其它 `[END_TOPIC]` → normal。
pub fn winding_reason_after_llm(
    user_busy_tag: bool,
    is_end_topic: bool,
    availability: Availability,
) -> Option<WindingReason> {
    if user_busy_tag {
        return Some(WindingReason::UserBusy);
    }
    if is_end_topic {
        if availability == Availability::Low {
            return Some(WindingReason::CharBusy);
        }
        return Some(WindingReason::Normal);
    }
    None
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
    fn paused_variants_resume_on_substantive() {
        for status in [
            ConversationStatus::Paused,
            ConversationStatus::PausedCharBusy,
            ConversationStatus::PausedUserBusy,
        ] {
            let decision = on_user_message(status, "今天好累啊", true);
            assert_eq!(
                decision,
                UserMessageDecision::CallLlm {
                    status: Some(ConversationStatus::Active)
                }
            );
        }
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
    fn winding_reason_from_end_topic_and_user_busy() {
        assert_eq!(
            winding_reason_after_llm(false, true, Availability::Low),
            Some(WindingReason::CharBusy)
        );
        assert_eq!(
            winding_reason_after_llm(false, true, Availability::High),
            Some(WindingReason::Normal)
        );
        assert_eq!(
            winding_reason_after_llm(true, true, Availability::Low),
            Some(WindingReason::UserBusy)
        );
        assert_eq!(
            winding_reason_after_llm(false, false, Availability::Low),
            None
        );
    }

    #[test]
    fn reason_maps_to_terminal() {
        assert_eq!(
            WindingReason::Normal.terminal_status(),
            ConversationStatus::Paused
        );
        assert_eq!(
            WindingReason::CharBusy.terminal_status(),
            ConversationStatus::PausedCharBusy
        );
        assert_eq!(
            WindingReason::UserBusy.terminal_status(),
            ConversationStatus::PausedUserBusy
        );
    }
}
