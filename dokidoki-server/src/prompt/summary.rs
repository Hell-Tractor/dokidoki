use crate::llm::{ChatRequest, LlmMessage};

use super::templates::{t20_first_user, t20_merge_user, t20_system};

pub fn build_summary_request(
    conversation_id: &str,
    existing_summary: Option<&str>,
    messages_to_summarize: &str,
    max_summary_chars: u32,
) -> ChatRequest {
    let system = t20_system(max_summary_chars);
    let user = if let Some(existing) = existing_summary.filter(|value| !value.is_empty()) {
        t20_merge_user(existing, messages_to_summarize, max_summary_chars)
    } else {
        t20_first_user(messages_to_summarize)
    };

    ChatRequest {
        conversation_id: conversation_id.to_owned(),
        turn_id: "summary".into(),
        messages: vec![
            LlmMessage {
                role: "system".into(),
                content: system,
            },
            LlmMessage {
                role: "user".into(),
                content: user,
            },
        ],
        // 摘要是自由文本，关闭 JSON 模式。
        response_format: "off".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t20_system_uses_configured_char_limit() {
        let request = build_summary_request("conv", None, "用户: hi", 500);
        let system = &request.messages[0].content;
        assert!(system.contains("500 字以内"));
    }
}
