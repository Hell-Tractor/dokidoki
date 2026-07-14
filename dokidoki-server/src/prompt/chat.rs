use super::templates::{
    T01, T02, T03, T04_EMPTY, T04_WITH_MEMORIES, T05, T12, T13, T14, T15, T18, T19, T21,
};
use crate::domain::persona::Persona;
use crate::domain::Availability;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentStatePrompt {
    pub weekday_zh: String,
    pub time_hm: String,
    pub activity: String,
    pub mood: String,
    pub availability: Availability,
    pub random_event: Option<String>,
}

pub fn build_system_prompt(
    persona: &Persona,
    character_name: &str,
    user_display_name: &str,
    current_state: Option<&CurrentStatePrompt>,
    memories: &[String],
    summary: Option<&str>,
) -> String {
    let traits = persona.traits_joined("、");
    let tone = persona.speech_style.tone.as_str();
    let catchphrases = persona.catchphrases_joined("、");
    let forbidden = persona.forbidden_joined("、");
    let skip_reply_tendency = persona.conversation_behavior.skip_reply_tendency.as_str();
    let user_display_name = if user_display_name.trim().is_empty() {
        "你"
    } else {
        user_display_name.trim()
    };

    let t01 = T01
        .replace("{name}", character_name)
        .replace("{traits}", &traits)
        .replace("{tone}", tone)
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
    if let Some(style) = persona
        .conversation_style
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        parts.push(format!("【性格倾向】\n{style}"));
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
    persona: &Persona,
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

/// 主动消息场景附加（System 后半）；按触发类型拼接 T-13～T-21。
pub fn format_proactive_scene(
    trigger: &str,
    special_date_detail: Option<&str>,
    ask_user_busy_care: bool,
) -> String {
    let mut parts = Vec::new();
    match trigger {
        "pre_sleep" => {
            parts.push(T21.to_owned());
            if ask_user_busy_care {
                parts.push(
                    "【附加关心】你们因你去忙而中断过。可按性格决定是否轻轻问对方忙完了没；\
                     不要盘问，一句带过即可，晚安仍是主线。"
                        .into(),
                );
            }
        }
        "daily_greeting" => {
            parts.push(T13.to_owned());
            if let Some(detail) = special_date_detail.filter(|d| !d.is_empty()) {
                parts.push(T18.replace("{special_date_detail}", detail));
            }
        }
        "re_engage" => parts.push(T15.to_owned()),
        "silence_wake" => parts.push(T14.to_owned()),
        _ => {
            parts.push(format!(
                "【主动场景】\n你正在主动找对方说话（触发：{trigger}）。语气符合人设与当前状态。"
            ));
        }
    }
    parts.join("\n\n")
}

pub fn format_proactive_user_message(trigger: &str) -> String {
    T12.replace("{proactive_trigger}", trigger)
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
        .replace("{availability}", state.availability.as_str())
        .replace("{random_event_block}", random_event_block.trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::persona::{ConversationBehavior, Persona, SpeechStyle};

    fn persona_fixture() -> Persona {
        Persona {
            personality_traits: vec!["黏人".into(), "撒娇".into()],
            speech_style: SpeechStyle {
                tone: "甜美".into(),
                catchphrases: vec!["哥哥".into()],
                forbidden: vec!["像客服一样说话".into()],
            },
            conversation_behavior: ConversationBehavior {
                skip_reply_tendency: "low".into(),
                ..ConversationBehavior::default()
            },
            ..Persona::default()
        }
    }

    #[test]
    fn builds_prompt_from_persona() {
        let prompt = build_system_prompt(&persona_fixture(), "小爱", "阿明", None, &[], None);

        assert!(prompt.contains("你是 小爱"));
        assert!(prompt.contains("黏人、撒娇"));
        assert!(prompt.contains("你称呼对方为「阿明」"));
        assert!(prompt.contains("[REPLY]"));
        assert!(prompt.contains("skip_reply 倾向：low"));
    }

    #[test]
    fn empty_user_display_name_falls_back_to_ni() {
        let prompt = build_system_prompt(&Persona::default(), "小咲", "", None, &[], None);

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
            availability: Availability::Low,
            random_event: Some("电脑坏了".into()),
        };
        let prompt = build_system_prompt(&Persona::default(), "小咲", "阿明", Some(&state), &[], None);

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
            availability: Availability::Medium,
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
        let prompt = build_icebreaker_system_prompt(&Persona::default(), "小咲", "阿明", None);
        assert!(prompt.contains("【场景：第一次见面】"));
        assert!(prompt.contains("你第一次和 阿明 说话"));
    }

    #[test]
    fn injects_conversation_style_when_present() {
        let mut persona = Persona::default();
        persona.conversation_style = Some("比较在意对方".into());
        let prompt = build_system_prompt(&persona, "小咲", "阿明", None, &[], None);
        assert!(prompt.contains("【性格倾向】"));
        assert!(prompt.contains("比较在意对方"));
    }

    #[test]
    fn proactive_daily_greeting_scene_includes_t13_and_optional_t18() {
        let scene = format_proactive_scene("daily_greeting", None, false);
        assert!(scene.contains("每日问候"));
        assert!(!scene.contains("特殊日期"));

        let with_special =
            format_proactive_scene("daily_greeting", Some("对方生日（07-11）"), false);
        assert!(with_special.contains("每日问候"));
        assert!(with_special.contains("特殊日期"));
        assert!(with_special.contains("对方生日（07-11）"));
    }

    #[test]
    fn proactive_re_engage_scene_uses_t15() {
        let scene = format_proactive_scene("re_engage", None, false);
        assert!(scene.contains("话题重启"));
        assert!(scene.contains("paused"));
    }

    #[test]
    fn proactive_silence_wake_scene_uses_t14() {
        let scene = format_proactive_scene("silence_wake", None, false);
        assert!(scene.contains("沉默唤醒"));
        assert!(scene.contains("很久没回"));
    }

    #[test]
    fn proactive_pre_sleep_scene_uses_t21_and_optional_care() {
        let scene = format_proactive_scene("pre_sleep", None, false);
        assert!(scene.contains("睡前晚安"));
        assert!(!scene.contains("附加关心"));

        let with_care = format_proactive_scene("pre_sleep", None, true);
        assert!(with_care.contains("睡前晚安"));
        assert!(with_care.contains("附加关心"));
        assert!(with_care.contains("忙完"));
    }
}
