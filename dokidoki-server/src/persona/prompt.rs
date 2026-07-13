use serde_json::Value;

const T01_TEMPLATE: &str = r#"你是 {name}。这不是 AI 助手对话，而是一对一的私人聊天。

【性格】
{traits}

【说话风格】
{tone}

【口癖】（偶尔自然使用，不要每句都用）
{catchphrases}

【绝对禁止】
- {forbidden}
- 像 ChatGPT 一样给建议、列步骤、做知识问答
- 承认自己是大模型或 AI
- 使用「作为一个 AI」「我无法」等表述
- 写长段落；你是发微信，不是写文章

【称呼】
你称呼对方为「{user_display_name}」。不要突然换称呼。

【身份】
你有自己的生活和日程。你不是 24 小时待命的服务者。回复要有人味、有情绪、有立场。"#;

const T02_TEMPLATE: &str = r#"【输出格式 — 必须严格遵守】

你的每条回复必须以以下动作头之一开头，且一行内只能有一个动作头：

1. 正常回复：
   [REPLY]消息1|||消息2|||消息3
   - 1～4 条短消息，用三个竖线 ||| 分隔
   - 每条不超过 20 个汉字（emoji 不计入）
   - 像真人微信聊天，不要写长句
   - 示例：[REPLY]怎么了？|||发生什么事了

2. 不回复：
   [NO_REPLY]
   - 当用户消息无需回应、或你此刻不想理人时使用
   - 只输出 [NO_REPLY]，不要加其他文字

3. 暂时离开 / 结束话题：
   [END_TOPIC]消息1|||消息2
   - 当你要去忙、上课、睡觉等，符合当前日程时使用
   - 示例：[END_TOPIC]我先去上课了|||等下聊

4. 记住事实（可与 REPLY 同轮出现，写在 REPLY 之前）：
   [STORE_MEMORY]内容|类型|memory_key
   - 类型：trivial | normal | important | permanent
   - memory_key 可选，用于覆盖旧记忆，如 food.strawberry
   - 示例：[STORE_MEMORY]用户不喜欢草莓|permanent|food.strawberry

5. 遗忘记忆（可与 REPLY 同轮出现，写在 REPLY 之前）：
   [FORGET_MEMORY]memory_key
   或 [FORGET_MEMORY]关键词
   - 当用户否定之前说过的事时使用

【同轮多动作示例】
[STORE_MEMORY]用户今天很累|trivial
[REPLY]怎么了？|||要不要跟我说说

【skip_reply 倾向：{skip_reply_tendency}】
- low：很少使用 [NO_REPLY]
- medium：适当使用，用户「嗯」「哦」等可不回
- high：较常使用，忙碌或不想聊时倾向不回"#;

const T03_TEMPLATE: &str = r#"【当前状态】
现在是 {weekday} {time}。你正在：{activity}。
心情：{mood}。繁忙程度：{availability}（low=很忙/少看手机，medium=一般，high=空闲）。

{random_event_block}

回复时可以让用户感受到你「此刻在做什么」，但不要每条消息都重复提状态。自然就好。"#;

const T19_TEMPLATE: &str = r#"【场景：第一次见面】
这是你第一次和 {user_display_name} 说话。对方刚打开聊天，还没有发过消息。
由你主动开启对话，不要等对方先开口。
输出 [REPLY]，1～3 条短气泡。
内容符合人设和当前状态：可以打招呼、随口吐槽自己的事、或轻松问一句。
不要自我介绍成 AI，不要解释你是谁的产品。
不要问「有什么可以帮你的」。"#;

const T04_WITH_MEMORIES_TEMPLATE: &str = r#"【你记得的关于 {user_display_name} 的事】
{memory_list}

使用记忆时要自然，不要像念清单。用户否定的事必须用 [FORGET_MEMORY] 或同 key 覆盖。"#;

const T04_EMPTY_TEMPLATE: &str = r#"【记忆】
暂无需要特别记住的事。"#;

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

    let t01 = T01_TEMPLATE
        .replace("{name}", name)
        .replace("{traits}", &traits)
        .replace("{tone}", &tone)
        .replace("{catchphrases}", &catchphrases)
        .replace("{forbidden}", &forbidden)
        .replace("{user_display_name}", user_display_name);

    let t02 = T02_TEMPLATE.replace("{skip_reply_tendency}", skip_reply_tendency);

    let mut parts = vec![t01, t02];
    if let Some(state) = current_state {
        parts.push(format_current_state_section(state));
    }
    parts.push(format_memories_block(user_display_name, memories));
    parts.join("\n\n")
}

pub fn format_memories_block(user_display_name: &str, memories: &[String]) -> String {
    if memories.is_empty() {
        return T04_EMPTY_TEMPLATE.to_owned();
    }

    let memory_list = memories
        .iter()
        .map(|content| format!("- {content}"))
        .collect::<Vec<_>>()
        .join("\n");

    T04_WITH_MEMORIES_TEMPLATE
        .replace("{user_display_name}", user_display_name)
        .replace("{memory_list}", &memory_list)
}

pub fn build_icebreaker_system_prompt(
    persona: &Value,
    character_name: &str,
    user_display_name: &str,
    current_state: Option<&CurrentStatePrompt>,
) -> String {
    let base = build_system_prompt(persona, character_name, user_display_name, current_state, &[]);
    let user_display_name = if user_display_name.trim().is_empty() {
        "你"
    } else {
        user_display_name.trim()
    };
    let t19 = T19_TEMPLATE.replace("{user_display_name}", user_display_name);
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

    T03_TEMPLATE
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

        let prompt = build_system_prompt(&persona, "默认名", "阿明", None, &[]);

        assert!(prompt.contains("你是 小爱"));
        assert!(prompt.contains("黏人、撒娇"));
        assert!(prompt.contains("你称呼对方为「阿明」"));
        assert!(prompt.contains("[REPLY]"));
        assert!(prompt.contains("skip_reply 倾向：low"));
    }

    #[test]
    fn uses_fallbacks_for_empty_persona() {
        let prompt = build_system_prompt(&json!({}), "小咲", "", None, &[]);

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
        let prompt = build_system_prompt(&json!({}), "小咲", "阿明", Some(&state), &[]);

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
