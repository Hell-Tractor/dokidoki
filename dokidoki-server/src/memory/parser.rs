use crate::chat::parser::{parse_action, LlmAction};

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
    /// 同轮是否出现 `[USER_BUSY]` 标记。
    pub user_busy: bool,
    pub action: LlmAction,
}

pub fn parse_llm_response(raw: &str) -> ParsedLlmResponse {
    let mut store_memories = Vec::new();
    let mut forget_memories = Vec::new();
    let mut action_lines = Vec::new();
    let mut user_busy = false;

    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line == "[USER_BUSY]" || line.starts_with("[USER_BUSY]") {
            user_busy = true;
            continue;
        }
        if let Some(payload) = line.strip_prefix("[STORE_MEMORY]") {
            if let Some(action) = parse_store_memory(payload.trim()) {
                store_memories.push(action);
            } else {
                tracing::debug!(
                    payload = %payload.trim(),
                    "malformed STORE_MEMORY line dropped"
                );
            }
            continue;
        }
        if let Some(target) = line.strip_prefix("[FORGET_MEMORY]") {
            let target = target.trim();
            if target.is_empty() {
                tracing::debug!("empty FORGET_MEMORY target dropped");
            } else {
                forget_memories.push(ForgetMemoryAction {
                    target: target.to_owned(),
                });
            }
            continue;
        }
        action_lines.push(line);
    }

    let action = parse_action(&action_lines.join("\n"));
    ParsedLlmResponse {
        store_memories,
        forget_memories,
        user_busy,
        action,
    }
}

fn parse_store_memory(payload: &str) -> Option<StoreMemoryAction> {
    if payload.is_empty() {
        return None;
    }

    let parts: Vec<&str> = payload.split('|').map(str::trim).collect();
    let content = parts.first()?.to_string();
    if content.is_empty() {
        return None;
    }

    let (memory_type, memory_key) = match parts.len() {
        1 => (MemoryType::Normal, None),
        2 => {
            if let Some(kind) = MemoryType::parse(parts[1]) {
                (kind, None)
            } else {
                (MemoryType::Normal, Some(parts[1].to_owned()))
            }
        }
        _ => {
            let kind = MemoryType::parse(parts[1]).unwrap_or_default();
            let key = parts[2];
            if key.is_empty() {
                (kind, None)
            } else {
                (kind, Some(key.to_owned()))
            }
        }
    };

    Some(StoreMemoryAction {
        content,
        memory_type,
        memory_key,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_store_and_reply() {
        let parsed = parse_llm_response(
            "[STORE_MEMORY]用户不喜欢草莓|permanent|food.strawberry\n[REPLY]知道了|||以后不买",
        );
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
        let parsed = parse_llm_response("[FORGET_MEMORY]food.strawberry\n[REPLY]好");
        assert_eq!(parsed.forget_memories.len(), 1);
        assert_eq!(parsed.forget_memories[0].target, "food.strawberry");
    }

    #[test]
    fn parses_trivial_without_key() {
        let parsed = parse_llm_response("[STORE_MEMORY]用户今天很累|trivial\n[NO_REPLY]");
        assert_eq!(parsed.store_memories[0].memory_type, MemoryType::Trivial);
        assert!(parsed.store_memories[0].memory_key.is_none());
        assert_eq!(parsed.action, LlmAction::NoReply);
        assert!(!parsed.user_busy);
    }

    #[test]
    fn parses_user_busy_with_reply() {
        let parsed = parse_llm_response("[USER_BUSY]\n[REPLY]好，你先去忙");
        assert!(parsed.user_busy);
        assert!(matches!(parsed.action, LlmAction::Reply(_)));
    }
}
