//! 解析 LLM JSON 回合输出。

use serde::Deserialize;

use crate::chat::parser::LlmAction;

use super::types::MemoryType;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreMemoryAction {
    pub content: String,
    pub memory_type: MemoryType,
    pub memory_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForgetMemoryAction {
    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedLlmResponse {
    pub store_memories: Vec<StoreMemoryAction>,
    pub forget_memories: Vec<ForgetMemoryAction>,
    /// 同轮是否标记对方去忙。
    pub user_busy: bool,
    pub action: LlmAction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    InvalidJson(String),
    InvalidAction(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidJson(msg) => write!(f, "invalid json: {msg}"),
            Self::InvalidAction(msg) => write!(f, "invalid action: {msg}"),
        }
    }
}

#[derive(Debug, Deserialize)]
struct TurnJson {
    #[serde(default)]
    user_busy: bool,
    #[serde(default)]
    store_memories: Vec<StoreMemoryJson>,
    #[serde(default)]
    forget_memories: Vec<ForgetMemoryJson>,
    action: ActionJson,
}

#[derive(Debug, Deserialize)]
struct StoreMemoryJson {
    content: String,
    memory_type: String,
    #[serde(default)]
    memory_key: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ForgetMemoryJson {
    target: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ActionJson {
    NoReply,
    Reply { bubbles: Vec<String> },
    EndTopic { bubbles: Vec<String> },
}

/// 解析模型输出为结构化回合；失败返回错误（由上层打回重试）。
pub fn parse_llm_response(raw: &str) -> Result<ParsedLlmResponse, ParseError> {
    let json_text = extract_json_payload(raw).ok_or_else(|| {
        ParseError::InvalidJson("empty or non-json output".into())
    })?;

    let turn: TurnJson = serde_json::from_str(json_text).map_err(|err| {
        ParseError::InvalidJson(err.to_string())
    })?;

    let store_memories = turn
        .store_memories
        .into_iter()
        .filter_map(|item| {
            let content = item.content.trim();
            if content.is_empty() {
                tracing::debug!("empty store_memory content dropped");
                return None;
            }
            let memory_type = MemoryType::parse(&item.memory_type).unwrap_or_default();
            let memory_key = item
                .memory_key
                .map(|k| k.trim().to_owned())
                .filter(|k| !k.is_empty());
            Some(StoreMemoryAction {
                content: content.to_owned(),
                memory_type,
                memory_key,
            })
        })
        .collect();

    let forget_memories = turn
        .forget_memories
        .into_iter()
        .filter_map(|item| {
            let target = item.target.trim();
            if target.is_empty() {
                tracing::debug!("empty forget_memory target dropped");
                return None;
            }
            Some(ForgetMemoryAction {
                target: target.to_owned(),
            })
        })
        .collect();

    let action = match turn.action {
        ActionJson::NoReply => LlmAction::NoReply,
        ActionJson::Reply { bubbles } => {
            let bubbles = normalize_bubbles(bubbles);
            if bubbles.is_empty() {
                return Err(ParseError::InvalidAction(
                    "reply requires non-empty bubbles".into(),
                ));
            }
            LlmAction::Reply(bubbles)
        }
        ActionJson::EndTopic { bubbles } => {
            let bubbles = normalize_bubbles(bubbles);
            if bubbles.is_empty() {
                return Err(ParseError::InvalidAction(
                    "end_topic requires non-empty bubbles".into(),
                ));
            }
            LlmAction::EndTopic(bubbles)
        }
    };

    Ok(ParsedLlmResponse {
        store_memories,
        forget_memories,
        user_busy: turn.user_busy,
        action,
    })
}

fn normalize_bubbles(bubbles: Vec<String>) -> Vec<String> {
    bubbles
        .into_iter()
        .map(|b| b.trim().to_owned())
        .filter(|b| !b.is_empty())
        .collect()
}

/// 去掉可选的 ``` 围栏噪声，截取首尾 `{...}`。
fn extract_json_payload(raw: &str) -> Option<&str> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}')?;
    if end < start {
        return None;
    }
    Some(trimmed[start..=end].trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_store_and_reply() {
        let raw = r#"{
            "user_busy": false,
            "store_memories": [{
                "content": "用户不喜欢草莓",
                "memory_type": "permanent",
                "memory_key": "food.strawberry"
            }],
            "forget_memories": [],
            "action": { "type": "reply", "bubbles": ["知道了", "以后不买"] }
        }"#;
        let parsed = parse_llm_response(raw).unwrap();
        assert_eq!(parsed.store_memories.len(), 1);
        assert_eq!(parsed.store_memories[0].content, "用户不喜欢草莓");
        assert_eq!(parsed.store_memories[0].memory_type, MemoryType::Permanent);
        assert_eq!(
            parsed.store_memories[0].memory_key.as_deref(),
            Some("food.strawberry")
        );
        assert!(matches!(parsed.action, LlmAction::Reply(_)));
    }

    #[test]
    fn parses_forget_memory() {
        let raw = r#"{
            "forget_memories": [{"target": "food.strawberry"}],
            "action": { "type": "reply", "bubbles": ["好"] }
        }"#;
        let parsed = parse_llm_response(raw).unwrap();
        assert_eq!(parsed.forget_memories.len(), 1);
        assert_eq!(parsed.forget_memories[0].target, "food.strawberry");
    }

    #[test]
    fn parses_trivial_and_no_reply() {
        let raw = r#"{
            "store_memories": [{"content": "用户今天很累", "memory_type": "trivial", "memory_key": null}],
            "action": { "type": "no_reply" }
        }"#;
        let parsed = parse_llm_response(raw).unwrap();
        assert_eq!(parsed.store_memories[0].memory_type, MemoryType::Trivial);
        assert!(parsed.store_memories[0].memory_key.is_none());
        assert_eq!(parsed.action, LlmAction::NoReply);
        assert!(!parsed.user_busy);
    }

    #[test]
    fn parses_user_busy_with_reply() {
        let raw = r#"{
            "user_busy": true,
            "action": { "type": "reply", "bubbles": ["好，你先去忙"] }
        }"#;
        let parsed = parse_llm_response(raw).unwrap();
        assert!(parsed.user_busy);
        assert!(matches!(parsed.action, LlmAction::Reply(_)));
    }

    #[test]
    fn strips_markdown_fence() {
        let raw = "```json\n{\"action\":{\"type\":\"reply\",\"bubbles\":[\"嗨\"]}}\n```";
        let parsed = parse_llm_response(raw).unwrap();
        assert_eq!(
            parsed.action,
            LlmAction::Reply(vec!["嗨".into()])
        );
    }
}
