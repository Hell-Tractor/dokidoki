use super::templates::{
    T01, T02, T03, T04_EMPTY, T04_WITH_MEMORIES, T05, T06_HIGH, T06_LOW, T06_MEDIUM, T07, T09,
    T10, T12, T13, T14, T15, T15_CHAR_BUSY, T15_USER_BUSY, T16, T18, T19, T21,
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

/// 对话回复场景附加。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ChatSceneFlags {
    /// 对话回复路径：常驻注入 T-07，由模型自行判断低信息输入。
    pub is_chat_reply: bool,
    pub winding_down: bool,
}

pub fn build_system_prompt(
    persona: &Persona,
    character_name: &str,
    user_display_name: &str,
    current_state: Option<&CurrentStatePrompt>,
    memories: &[String],
    summary: Option<&str>,
) -> String {
    build_system_prompt_with_scenes(
        persona,
        character_name,
        user_display_name,
        current_state,
        memories,
        summary,
        ChatSceneFlags::default(),
    )
}

pub fn build_system_prompt_with_scenes(
    persona: &Persona,
    character_name: &str,
    user_display_name: &str,
    current_state: Option<&CurrentStatePrompt>,
    memories: &[String],
    summary: Option<&str>,
    scenes: ChatSceneFlags,
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
    if let Some(state) = current_state {
        parts.push(format_availability_style(state.availability));
    }
    if let Some(style) = persona
        .conversation_style
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        parts.push(T10.replace("{conversation_style}", style));
    }
    if let Some(scene) = format_chat_scenes(scenes) {
        parts.push(scene);
    }
    parts.join("\n\n")
}

pub fn format_availability_style(availability: Availability) -> String {
    match availability {
        Availability::Low => T06_LOW.to_owned(),
        Availability::Medium => T06_MEDIUM.to_owned(),
        Availability::High => T06_HIGH.to_owned(),
    }
}

pub fn format_chat_scenes(scenes: ChatSceneFlags) -> Option<String> {
    let mut parts = Vec::new();
    if scenes.is_chat_reply {
        parts.push(T07.to_owned());
    }
    if scenes.winding_down {
        parts.push(T09.to_owned());
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n\n"))
    }
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
    schedule_change: Option<(&str, Option<&str>)>,
    re_engage_reason: Option<&str>,
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
        "schedule_change" => {
            let (current, previous) = schedule_change.unwrap_or(("", None));
            let previous_block = previous
                .filter(|p| !p.is_empty())
                .map(|p| format!("上一档活动是：{p}。\n"))
                .unwrap_or_default();
            parts.push(
                T16.replace("{current_activity}", current)
                    .replace("{previous_activity_block}", previous_block.trim_end()),
            );
        }
        "re_engage" => {
            parts.push(T15.to_owned());
            match re_engage_reason {
                Some("char_busy") => parts.push(T15_CHAR_BUSY.to_owned()),
                Some("user_busy") => parts.push(T15_USER_BUSY.to_owned()),
                _ => {}
            }
        }
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
        assert!(prompt.contains("此刻回复风格 — 忙碌"));
    }

    #[test]
    fn availability_style_varies_by_level() {
        assert!(format_availability_style(Availability::Low).contains("忙碌"));
        assert!(format_availability_style(Availability::Medium).contains("一般"));
        assert!(format_availability_style(Availability::High).contains("空闲"));
    }

    #[test]
    fn chat_scenes_include_t07_for_chat_reply_and_optional_t09() {
        let reply_only = format_chat_scenes(ChatSceneFlags {
            is_chat_reply: true,
            winding_down: false,
        })
        .expect("scenes");
        assert!(reply_only.contains("低信息输入处理"));
        assert!(!reply_only.contains("话题收尾中"));

        let both = format_chat_scenes(ChatSceneFlags {
            is_chat_reply: true,
            winding_down: true,
        })
        .expect("scenes");
        assert!(both.contains("低信息输入处理"));
        assert!(both.contains("话题收尾中"));

        assert!(format_chat_scenes(ChatSceneFlags::default()).is_none());
    }

    #[test]
    fn chat_prompt_always_includes_low_info_guidance() {
        let prompt = build_system_prompt_with_scenes(
            &Persona::default(),
            "小咲",
            "阿明",
            None,
            &[],
            None,
            ChatSceneFlags {
                is_chat_reply: true,
                winding_down: true,
            },
        );
        assert!(prompt.contains("低信息输入处理"));
        assert!(prompt.contains("陪伴模式"));
        assert!(prompt.contains("话题收尾中"));
        assert!(prompt.contains("若对方消息有实质内容"));
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
        let scene = format_proactive_scene("daily_greeting", None, false, None, None);
        assert!(scene.contains("每日问候"));
        assert!(!scene.contains("特殊日期"));

        let with_special =
            format_proactive_scene("daily_greeting", Some("对方生日（07-11）"), false, None, None);
        assert!(with_special.contains("每日问候"));
        assert!(with_special.contains("特殊日期"));
        assert!(with_special.contains("对方生日（07-11）"));
    }

    #[test]
    fn proactive_re_engage_scene_uses_t15_and_reason() {
        let base = format_proactive_scene("re_engage", None, false, None, None);
        assert!(base.contains("话题重启"));
        assert!(!base.contains("重启原因"));

        let char_busy =
            format_proactive_scene("re_engage", None, false, None, Some("char_busy"));
        assert!(char_busy.contains("话题重启"));
        assert!(char_busy.contains("你去忙刚结束"));
        assert!(char_busy.contains("忙完啦"));

        let user_busy =
            format_proactive_scene("re_engage", None, false, None, Some("user_busy"));
        assert!(user_busy.contains("话题重启"));
        assert!(user_busy.contains("对方去忙"));
        assert!(user_busy.contains("催促"));
    }

    #[test]
    fn proactive_silence_wake_scene_uses_t14() {
        let scene = format_proactive_scene("silence_wake", None, false, None, None);
        assert!(scene.contains("沉默唤醒"));
        assert!(scene.contains("很久没回"));
    }

    #[test]
    fn proactive_pre_sleep_scene_uses_t21_and_optional_care() {
        let scene = format_proactive_scene("pre_sleep", None, false, None, None);
        assert!(scene.contains("睡前晚安"));
        assert!(!scene.contains("附加关心"));

        let with_care = format_proactive_scene("pre_sleep", None, true, None, None);
        assert!(with_care.contains("睡前晚安"));
        assert!(with_care.contains("附加关心"));
        assert!(with_care.contains("忙完"));
    }

    #[test]
    fn proactive_schedule_change_scene_uses_t16() {
        let scene = format_proactive_scene(
            "schedule_change",
            None,
            false,
            Some(("回家做饭", Some("录音棚配音"))),
            None,
        );
        assert!(scene.contains("日程切换"));
        assert!(scene.contains("回家做饭"));
        assert!(scene.contains("录音棚配音"));
    }
}
