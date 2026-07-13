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

【skip_reply 倾向：{skip_reply_tendency}】
- low：很少使用 [NO_REPLY]
- medium：适当使用，用户「嗯」「哦」等可不回
- high：较常使用，忙碌或不想聊时倾向不回"#;

pub fn build_system_prompt(
    persona: &Value,
    character_name: &str,
    user_display_name: &str,
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

    format!("{t01}\n\n{t02}")
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

        let prompt = build_system_prompt(&persona, "默认名", "阿明");

        assert!(prompt.contains("你是 小爱"));
        assert!(prompt.contains("黏人、撒娇"));
        assert!(prompt.contains("你称呼对方为「阿明」"));
        assert!(prompt.contains("[REPLY]"));
        assert!(prompt.contains("skip_reply 倾向：low"));
    }

    #[test]
    fn uses_fallbacks_for_empty_persona() {
        let prompt = build_system_prompt(&json!({}), "小咲", "");

        assert!(prompt.contains("你是 小咲"));
        assert!(prompt.contains("你称呼对方为「你」"));
    }
}
