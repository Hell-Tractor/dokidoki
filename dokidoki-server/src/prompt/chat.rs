use serde_json::Value;

use super::templates::{T01, T02, T03, T04_EMPTY, T04_WITH_MEMORIES, T05, T19};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentStatePrompt {
    pub weekday_zh: String,
    pub time_hm: String,
    pub activity: String,
    pub mood: String,
    pub availability: String,
    pub random_event: Option<String>,
}

pub fn build_system_prompt(
    persona: &Value,
    character_name: &str,
    user_display_name: &str,
    current_state: Option<&CurrentStatePrompt>,
    memories: &[String],
    summary: Option<&str>,
) -> String {
    let name = persona
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or(character_name);
    let traits = join_string_array(persona.get("personality_traits"), "、");
    let tone = nested_str(persona, &["speech_style", "tone"]);
    let catchphrases = join_string_array(
        persona
            .get("speech_style")
            .and_then(|v| v.get("catchphrases")),
        "、",
    );
    let forbidden = join_string_array(
        persona
            .get("speech_style")
            .and_then(|v| v.get("forbidden")),
        "、",
    );
    let skip_reply_tendency = persona
        .get("conversation_behavior")
        .and_then(|v| v.get("skip_reply_tendency"))
        .and_then(Value::as_str)
        .unwrap_or("medium");
    let user_display_name = if user_display_name.trim().is_empty() {
        "你"
    } else {
        user_display_name.trim()
    };

    let t01 = T01
        .replace("{name}", name)
        .replace("{traits}", &traits)
        .replace("{tone}", &tone)
        .replace("{catchphrases}", &catchphrases)
        .replace("{forbidden}", &forbidden)
        .replace("{user_display_name}", user_display_name);

    let t02 = T02.replace("{skip_reply_tendency}", skip_reply_tendency);

    let mut parts = vec![t01, t02];
    if let Some(state) = current_state {
        parts.push(format_current_state_section(state));
    }
    parts.push(format_memories_block(user_display_name, memories));
    if let Some(summary) = summary.filter(|value| !value.is_empty()) {
        parts.push(format_summary_block(summary));
    }
    parts.join("\n\n")
}

pub fn format_summary_block(summary: &str) -> String {
    T05.replace("{summary}", summary.trim())
}

pub fn format_memories_block(user_display_name: &str, memories: &[String]) -> String {
    if memories.is_empty() {
        return T04_EMPTY.to_owned();
    }

    let memory_list = memories
        .iter()
        .map(|content| format!("- {content}"))
        .collect::<Vec<_>>()
        .join("\n");

    T04_WITH_MEMORIES
        .replace("{user_display_name}", user_display_name)
        .replace("{memory_list}", &memory_list)
}

pub fn build_icebreaker_system_prompt(
    persona: &Value,
    character_name: &str,
    user_display_name: &str,
    current_state: Option<&CurrentStatePrompt>,
) -> String {
    let base = build_system_prompt(persona, character_name, user_display_name, current_state, &[], None);
    let user_display_name = if user_display_name.trim().is_empty() {
        "你"
    } else {
        user_display_name.trim()
    };
    let t19 = T19.replace("{user_display_name}", user_display_name);
    format!("{base}\n\n{t19}")
}

pub fn format_icebreaker_user_message() -> &'static str {
    "（系统）请发起初次对话。"
}

pub fn format_current_state_section(state: &CurrentStatePrompt) -> String {
    let random_event_block = state
        .random_event
        .as_ref()
        .filter(|s| !s.is_empty())
        .map(|event| format!("【今日变故】{event}\n"))
        .unwrap_or_default();

    T03
        .replace("{weekday}", &state.weekday_zh)
        .replace("{time}", &state.time_hm)
        .replace("{activity}", &state.activity)
        .replace("{mood}", &state.mood)
        .replace("{availability}", &state.availability)
        .replace("{random_event_block}", random_event_block.trim_end())
}

fn nested_str(value: &Value, path: &[&str]) -> String {
    let mut current = value;
    for key in path {
        current = match current.get(*key) {
            Some(v) => v,
            None => return String::new(),
        };
    }
    current.as_str().unwrap_or_default().to_owned()
}

fn join_string_array(value: Option<&Value>, sep: &str) -> String {
    match value.and_then(Value::as_array) {
        Some(items) => items
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(sep),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn builds_prompt_from_persona_json() {
        let persona = json!({
            "name": "小爱",
            "personality_traits": ["黏人", "撒娇"],
            "speech_style": {
                "tone": "甜美",
                "catchphrases": ["哥哥"],
                "forbidden": ["像客服一样说话"]
            },
            "conversation_behavior": {
                "skip_reply_tendency": "low"
            }
        });

        let prompt = build_system_prompt(&persona, "默认名", "阿明", None, &[], None);

        assert!(prompt.contains("你是 小爱"));
        assert!(prompt.contains("黏人、撒娇"));
        assert!(prompt.contains("你称呼对方为「阿明」"));
        assert!(prompt.contains("[REPLY]"));
        assert!(prompt.contains("skip_reply 倾向：low"));
    }

    #[test]
    fn uses_fallbacks_for_empty_persona() {
        let prompt = build_system_prompt(&json!({}), "小咲", "", None, &[], None);

        assert!(prompt.contains("你是 小咲"));
        assert!(prompt.contains("你称呼对方为「你」"));
    }

    #[test]
    fn appends_t03_when_current_state_provided() {
        let state = CurrentStatePrompt {
            weekday_zh: "周一".into(),
            time_hm: "10:00".into(),
            activity: "工作".into(),
            mood: "专注".into(),
            availability: "low".into(),
            random_event: Some("电脑坏了".into()),
        };
        let prompt = build_system_prompt(&json!({}), "小咲", "阿明", Some(&state), &[], None);

        assert!(prompt.contains("【当前状态】"));
        assert!(prompt.contains("你正在：工作"));
        assert!(prompt.contains("【今日变故】电脑坏了"));
    }

    #[test]
    fn t03_omits_random_event_block_when_empty() {
        let state = CurrentStatePrompt {
            weekday_zh: "周二".into(),
            time_hm: "08:00".into(),
            activity: "早餐".into(),
            mood: "元气".into(),
            availability: "medium".into(),
            random_event: None,
        };
        let section = format_current_state_section(&state);
        assert!(!section.contains("【今日变故】"));
    }

    #[test]
    fn t05_includes_summary_block() {
        let block = format_summary_block("用户和小咲聊了工作的事");
        assert!(block.contains("更早之前的聊天摘要"));
        assert!(block.contains("用户和小咲聊了工作的事"));
    }

    #[test]
    fn t04_includes_active_memories() {
        let memories = vec!["用户不喜欢草莓".to_owned(), "用户昨天很累".to_owned()];
        let block = format_memories_block("阿明", &memories);
        assert!(block.contains("用户不喜欢草莓"));
        assert!(block.contains("阿明"));
    }

    #[test]
    fn t04_shows_empty_placeholder() {
        let block = format_memories_block("阿明", &[]);
        assert!(block.contains("暂无需要特别记住的事"));
    }

    #[test]
    fn icebreaker_prompt_includes_t19() {
        let prompt = build_icebreaker_system_prompt(&json!({}), "小咲", "阿明", None);
        assert!(prompt.contains("【场景：第一次见面】"));
        assert!(prompt.contains("你第一次和 阿明 说话"));
    }
}
