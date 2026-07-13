//! 解析 LLM 动作头：`[REPLY]` / `[NO_REPLY]` / `[END_TOPIC]`。

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlmAction {
    NoReply,
    Reply(Vec<String>),
    EndTopic(Vec<String>),
}

pub fn parse_action(raw: &str) -> LlmAction {
    let text = raw.trim();
    if text.is_empty() || matches_no_reply(text) {
        return LlmAction::NoReply;
    }
    if let Some(rest) = strip_action_prefix(text, "END_TOPIC") {
        return LlmAction::EndTopic(split_bubbles(rest));
    }
    if let Some(rest) = strip_action_prefix(text, "REPLY") {
        return LlmAction::Reply(split_bubbles(rest));
    }
    LlmAction::Reply(split_bubbles(text))
}

/// 兼容破冰等仅关心 REPLY 气泡的场景。
pub fn parse_reply(raw: &str) -> Vec<String> {
    match parse_action(raw) {
        LlmAction::Reply(bubbles) | LlmAction::EndTopic(bubbles) => bubbles,
        LlmAction::NoReply => Vec::new(),
    }
}

fn matches_no_reply(text: &str) -> bool {
    text == "[NO_REPLY]" || text.starts_with("[NO_REPLY]")
}

fn strip_action_prefix<'a>(text: &'a str, action: &str) -> Option<&'a str> {
    let marker = format!("[{action}]");
    text.strip_prefix(&marker).map(str::trim)
}

fn split_bubbles(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    text.split("|||")
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_no_reply() {
        assert_eq!(parse_action("[NO_REPLY]"), LlmAction::NoReply);
        assert_eq!(parse_action(""), LlmAction::NoReply);
    }

    #[test]
    fn parses_single_reply() {
        assert_eq!(
            parse_action("[REPLY] 你好"),
            LlmAction::Reply(vec!["你好".to_owned()])
        );
    }

    #[test]
    fn parses_end_topic() {
        assert_eq!(
            parse_action("[END_TOPIC]我先走了|||等下聊"),
            LlmAction::EndTopic(vec!["我先走了".to_owned(), "等下聊".to_owned()])
        );
    }

    #[test]
    fn parses_multiple_bubbles() {
        assert_eq!(
            parse_reply("[REPLY] 第一句|||第二句"),
            vec!["第一句".to_owned(), "第二句".to_owned()]
        );
    }

    #[test]
    fn empty_reply_returns_empty_vec() {
        assert!(parse_reply("[REPLY]").is_empty());
    }

    #[test]
    fn fallback_treats_plain_text_as_reply() {
        assert_eq!(
            parse_action("你好"),
            LlmAction::Reply(vec!["你好".to_owned()])
        );
    }
}
