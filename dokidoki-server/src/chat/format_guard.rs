//! 气泡长度校验 + LLM 格式打回重试。

use crate::{
    chat::parser::LlmAction,
    error::AppError,
    llm::{ChatRequest, LlmClient, LlmMessage},
    memory::{parse_llm_response, ParseError, ParsedLlmResponse},
};

#[derive(Debug, Clone, Copy)]
pub struct FormatLimits {
    pub max_bubble_chars: usize,
    pub max_bubbles: usize,
    pub max_retries: u32,
}

impl FormatLimits {
    pub fn from_chat(config: &crate::config::Chat) -> Self {
        Self {
            max_bubble_chars: config.max_bubble_chars,
            max_bubbles: config.max_bubbles,
            max_retries: config.llm_format_retries,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatIssue {
    Parse(ParseError),
    Overlong {
        index: usize,
        len: usize,
        preview: String,
    },
    TooManyBubbles(usize),
}

impl std::fmt::Display for FormatIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(err) => write!(f, "{err}"),
            Self::Overlong {
                index,
                len,
                preview,
            } => write!(
                f,
                "bubble[{index}] too long ({len} chars): {preview}"
            ),
            Self::TooManyBubbles(n) => write!(f, "too many bubbles: {n}"),
        }
    }
}

/// 调用 LLM，解析 JSON，校验气泡长度；不合格则附带纠错 user 消息重试。
pub async fn chat_with_format_retry(
    llm: &LlmClient,
    mut request: ChatRequest,
    limits: FormatLimits,
) -> Result<ParsedLlmResponse, AppError> {
    let mut attempt = 0u32;
    loop {
        let raw = llm.chat(request.clone()).await?;
        match parse_and_validate(&raw, &limits) {
            Ok(parsed) => {
                if attempt > 0 {
                    tracing::info!(
                        conversation_id = %request.conversation_id,
                        turn_id = %request.turn_id,
                        attempt,
                        "llm format retry succeeded"
                    );
                }
                return Ok(parsed);
            }
            Err(issue) => {
                if attempt >= limits.max_retries {
                    tracing::warn!(
                        conversation_id = %request.conversation_id,
                        turn_id = %request.turn_id,
                        attempt,
                        issue = %issue,
                        "llm format retries exhausted; splitting overlong bubbles by punctuation"
                    );
                    return Ok(parse_with_bubble_split(&raw, &limits).unwrap_or_else(|_| {
                        ParsedLlmResponse {
                            store_memories: Vec::new(),
                            forget_memories: Vec::new(),
                            user_busy: false,
                            action: LlmAction::NoReply,
                        }
                    }));
                }
                tracing::info!(
                    conversation_id = %request.conversation_id,
                    turn_id = %request.turn_id,
                    attempt,
                    issue = %issue,
                    "llm format invalid; requesting regenerate"
                );
                request.messages.push(LlmMessage {
                    role: "assistant".into(),
                    content: raw,
                });
                request.messages.push(LlmMessage {
                    role: "user".into(),
                    content: retry_user_message(&issue, &limits),
                });
                attempt += 1;
            }
        }
    }
}

pub fn parse_and_validate(
    raw: &str,
    limits: &FormatLimits,
) -> Result<ParsedLlmResponse, FormatIssue> {
    let parsed = parse_llm_response(raw).map_err(FormatIssue::Parse)?;
    validate_bubbles(&parsed.action, limits)?;
    Ok(parsed)
}

fn validate_bubbles(action: &LlmAction, limits: &FormatLimits) -> Result<(), FormatIssue> {
    let bubbles = match action {
        LlmAction::NoReply => return Ok(()),
        LlmAction::Reply(b) | LlmAction::EndTopic(b) => b,
    };
    if bubbles.len() > limits.max_bubbles {
        return Err(FormatIssue::TooManyBubbles(bubbles.len()));
    }
    for (index, bubble) in bubbles.iter().enumerate() {
        let len = bubble_char_count(bubble);
        if len > limits.max_bubble_chars {
            return Err(FormatIssue::Overlong {
                index,
                len,
                preview: bubble.chars().take(40).collect(),
            });
        }
    }
    Ok(())
}

/// 重试耗尽后的兜底：按分隔类标点拆分超长气泡（不丢弃正文）。
fn parse_with_bubble_split(
    raw: &str,
    limits: &FormatLimits,
) -> Result<ParsedLlmResponse, FormatIssue> {
    let mut parsed = parse_llm_response(raw).map_err(FormatIssue::Parse)?;
    let bubbles = match &parsed.action {
        LlmAction::NoReply => return Ok(parsed),
        LlmAction::Reply(b) | LlmAction::EndTopic(b) => b.clone(),
    };

    let mut split: Vec<String> = Vec::new();
    for bubble in bubbles {
        for part in split_overlong_bubble(&bubble, limits.max_bubble_chars) {
            if !part.is_empty() {
                split.push(part);
            }
        }
    }
    split.truncate(limits.max_bubbles);

    parsed.action = match parsed.action {
        LlmAction::EndTopic(_) => {
            if split.is_empty() {
                LlmAction::NoReply
            } else {
                LlmAction::EndTopic(split)
            }
        }
        LlmAction::Reply(_) | LlmAction::NoReply => {
            if split.is_empty() {
                LlmAction::NoReply
            } else {
                LlmAction::Reply(split)
            }
        }
    };
    Ok(parsed)
}

/// 超长则按分隔标点拆开；无分隔标点或拆后仍超长则整段保留（不强行按字数切）。
fn split_overlong_bubble(text: &str, max_chars: usize) -> Vec<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    if bubble_char_count(trimmed) <= max_chars {
        return vec![trimmed.to_owned()];
    }

    let parts = split_by_separator_punct(trimmed);
    if parts.len() <= 1 {
        // 无可拆分隔标点：保留超长原文。
        return vec![trimmed.to_owned()];
    }

    let mut out = Vec::new();
    for part in parts {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        // 拆开后若某段仍超长，同样整段保留，不再按字数硬切。
        out.push(part.to_owned());
    }
    if out.is_empty() {
        vec![trimmed.to_owned()]
    } else {
        out
    }
}

/// 分隔类标点（不带语气）：中英文逗号/句号/分号/顿号/冒号。拆分时丢弃该标点。
fn split_by_separator_punct(text: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    for c in text.chars() {
        if is_separator_punct(c) {
            let trimmed = current.trim().to_owned();
            if !trimmed.is_empty() {
                parts.push(trimmed);
            }
            current.clear();
            continue;
        }
        current.push(c);
    }
    let trimmed = current.trim().to_owned();
    if !trimmed.is_empty() {
        parts.push(trimmed);
    }
    parts
}

fn is_separator_punct(c: char) -> bool {
    matches!(
        c,
        '，' | '。' | '；' | '、' | '：' | ',' | '.' | ';' | ':'
    )
}

fn retry_user_message(issue: &FormatIssue, limits: &FormatLimits) -> String {
    format!(
        "上轮输出不合格（{issue}）。请只输出一个合法 JSON 对象（不要 Markdown）。\
         action.bubbles 至多 {max_bubbles} 条，每条不超过 {max_chars} 字（emoji 不计入）。\
         过长请拆成多条短气泡后重新输出完整 JSON。",
        max_bubbles = limits.max_bubbles,
        max_chars = limits.max_bubble_chars,
    )
}

/// 气泡字数：排除 emoji，其余字符（含标点、英文）计入。
pub fn bubble_char_count(text: &str) -> usize {
    text.chars().filter(|c| !is_emoji_char(*c)).count()
}

fn is_emoji_char(c: char) -> bool {
    matches!(c as u32,
        0x1F300..=0x1FAFF // misc emoji / symbols
        | 0x2600..=0x27BF  // misc symbols
        | 0xFE00..=0xFE0F  // variation selectors
        | 0x1F1E6..=0x1F1FF // flags
        | 0x200D           // ZWJ
        | 0x20E3           // combining enclosing keycap
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_ignore_emoji() {
        assert_eq!(bubble_char_count("你好😊"), 2);
        assert_eq!(bubble_char_count("hello"), 5);
    }

    #[test]
    fn rejects_overlong() {
        let limits = FormatLimits {
            max_bubble_chars: 5,
            max_bubbles: 4,
            max_retries: 1,
        };
        let raw = r#"{"action":{"type":"reply","bubbles":["这是超过五个字的句子"]}}"#;
        let err = parse_and_validate(raw, &limits).unwrap_err();
        assert!(matches!(err, FormatIssue::Overlong { .. }));
    }

    #[test]
    fn accepts_short_bubbles() {
        let limits = FormatLimits {
            max_bubble_chars: 20,
            max_bubbles: 4,
            max_retries: 1,
        };
        let raw = r#"{"action":{"type":"reply","bubbles":["怎么了？","发生什么事了"]}}"#;
        let parsed = parse_and_validate(raw, &limits).unwrap();
        assert!(matches!(parsed.action, LlmAction::Reply(_)));
    }

    #[test]
    fn splits_overlong_by_comma_and_period() {
        let limits = FormatLimits {
            max_bubble_chars: 8,
            max_bubbles: 4,
            max_retries: 0,
        };
        let raw = r#"{"action":{"type":"reply","bubbles":["今天天气不错，我们出去走走吧。"]}}"#;
        let parsed = parse_with_bubble_split(raw, &limits).unwrap();
        let LlmAction::Reply(bubbles) = parsed.action else {
            panic!("expected reply");
        };
        assert_eq!(bubbles, vec!["今天天气不错", "我们出去走走吧"]);
        for b in &bubbles {
            assert!(bubble_char_count(b) <= limits.max_bubble_chars);
        }
    }

    #[test]
    fn drops_separator_punct_on_split() {
        assert_eq!(
            split_by_separator_punct("aaaaa,bbbbb"),
            vec!["aaaaa", "bbbbb"]
        );
        assert_eq!(
            split_by_separator_punct("短，这是一段仍然很长很长很长的话。"),
            vec!["短", "这是一段仍然很长很长很长的话"]
        );
    }

    #[test]
    fn keeps_overlong_when_no_separator() {
        let text = "真的吗你说的是真的吗根本没法拆";
        let parts = split_overlong_bubble(text, 5);
        assert_eq!(parts, vec![text]);
    }

    #[test]
    fn keeps_overlong_segment_after_punct_split() {
        // 逗号后半段仍超长：保留该段，不强行切字；分隔标点丢弃。
        let parts = split_overlong_bubble("短，这是一段仍然很长很长很长的话", 5);
        assert_eq!(parts, vec!["短", "这是一段仍然很长很长很长的话"]);
    }
}
