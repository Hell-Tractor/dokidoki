//! LLM 动作枚举（解析后的领域类型）。

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlmAction {
    NoReply,
    Reply(Vec<String>),
    EndTopic(Vec<String>),
}

/// 从已解析回合取出气泡（破冰 / 主动消息用）。
pub fn bubbles_from_action(action: &LlmAction) -> Vec<String> {
    match action {
        LlmAction::Reply(bubbles) | LlmAction::EndTopic(bubbles) => bubbles.clone(),
        LlmAction::NoReply => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bubbles_from_reply() {
        assert_eq!(
            bubbles_from_action(&LlmAction::Reply(vec!["a".into()])),
            vec!["a".to_owned()]
        );
        assert!(bubbles_from_action(&LlmAction::NoReply).is_empty());
    }
}
