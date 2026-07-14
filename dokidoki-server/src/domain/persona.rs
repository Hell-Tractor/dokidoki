use serde::Deserialize;

/// 角色人设（`characters.persona_json`）。
/// 角色展示名使用表字段 `characters.name`，不在此重复存储。
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
pub struct Persona {
    #[serde(default)]
    pub personality_traits: Vec<String>,
    #[serde(default)]
    pub speech_style: SpeechStyle,
    #[serde(default)]
    pub reply_delay_factor: ReplyDelayFactor,
    #[serde(default)]
    pub conversation_behavior: ConversationBehavior,
    #[serde(default)]
    pub proactive: ProactiveConfig,
    #[serde(default)]
    pub conversation_style: Option<String>,
    #[serde(default)]
    pub emotional_triggers: EmotionalTriggers,
}

impl Persona {
    pub fn from_json_value(value: serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(value)
    }

    pub fn traits_joined(&self, sep: &str) -> String {
        self.personality_traits.join(sep)
    }

    pub fn catchphrases_joined(&self, sep: &str) -> String {
        self.speech_style.catchphrases.join(sep)
    }

    pub fn forbidden_joined(&self, sep: &str) -> String {
        self.speech_style.forbidden.join(sep)
    }
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct SpeechStyle {
    #[serde(default)]
    pub tone: String,
    #[serde(default)]
    pub catchphrases: Vec<String>,
    #[serde(default)]
    pub forbidden: Vec<String>,
}

/// 回复延迟性格系数区间；JSON 可为数字或 `[min, max]`。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReplyDelayFactor {
    pub min: f64,
    pub max: f64,
}

impl Default for ReplyDelayFactor {
    fn default() -> Self {
        Self { min: 1.0, max: 1.0 }
    }
}

impl ReplyDelayFactor {
    pub fn sample(self, random_unit: f64) -> f64 {
        crate::utils::uniform(self.min.min(self.max), self.min.max(self.max), random_unit)
    }
}

impl<'de> Deserialize<'de> for ReplyDelayFactor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        match value {
            serde_json::Value::Number(n) => {
                let v = n.as_f64().unwrap_or(1.0);
                Ok(Self { min: v, max: v })
            }
            serde_json::Value::Array(items) => match items.as_slice() {
                [] => Ok(Self::default()),
                [only] => {
                    let v = only.as_f64().unwrap_or(1.0);
                    Ok(Self { min: v, max: v })
                }
                [a, b, ..] => {
                    let min = a.as_f64().unwrap_or(1.0);
                    let max = b.as_f64().unwrap_or(min);
                    Ok(Self {
                        min: min.min(max),
                        max: min.max(max),
                    })
                }
            },
            _ => Ok(Self::default()),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ConversationBehavior {
    #[serde(default = "default_skip_reply_tendency")]
    pub skip_reply_tendency: String,
    #[serde(default)]
    pub end_topic_freely: bool,
    #[serde(default = "default_re_engage_after_minutes")]
    pub re_engage_after_minutes: u32,
    #[serde(default = "default_true")]
    pub pause_on_farewell: bool,
}

impl Default for ConversationBehavior {
    fn default() -> Self {
        Self {
            skip_reply_tendency: default_skip_reply_tendency(),
            end_topic_freely: false,
            re_engage_after_minutes: default_re_engage_after_minutes(),
            pause_on_farewell: true,
        }
    }
}

fn default_skip_reply_tendency() -> String {
    "medium".into()
}

fn default_re_engage_after_minutes() -> u32 {
    120
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ProactiveConfig {
    #[serde(default = "default_silence_after_hours")]
    pub silence_after_hours: f64,
    #[serde(default = "default_probability_factor")]
    pub probability_factor: f64,
}

impl Default for ProactiveConfig {
    fn default() -> Self {
        Self {
            silence_after_hours: default_silence_after_hours(),
            probability_factor: default_probability_factor(),
        }
    }
}

fn default_silence_after_hours() -> f64 {
    8.0
}

fn default_probability_factor() -> f64 {
    1.0
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct EmotionalTriggers {
    #[serde(default)]
    pub user_sad: Option<String>,
    #[serde(default)]
    pub user_shares_photo: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn deserializes_seed_shaped_persona() {
        let persona: Persona = serde_json::from_value(json!({
            "personality_traits": ["黏人"],
            "speech_style": { "tone": "甜美", "catchphrases": ["哥哥"], "forbidden": ["客服"] },
            "reply_delay_factor": [0.5, 0.7],
            "conversation_behavior": {
                "skip_reply_tendency": "low",
                "pause_on_farewell": false
            },
            "proactive": { "silence_after_hours": 4, "probability_factor": 1.2 },
            "conversation_style": "比较在意对方"
        }))
        .unwrap();

        assert_eq!(persona.personality_traits, vec!["黏人".to_owned()]);
        assert_eq!(persona.reply_delay_factor, ReplyDelayFactor { min: 0.5, max: 0.7 });
        assert!(!persona.conversation_behavior.pause_on_farewell);
        assert_eq!(persona.proactive.silence_after_hours, 4.0);
    }

    #[test]
    fn ignores_legacy_name_field_in_json() {
        let persona: Persona = serde_json::from_value(json!({ "name": "小爱" })).unwrap();
        assert_eq!(persona, Persona::default());
    }

    #[test]
    fn other_fields_use_defaults_when_omitted() {
        let persona: Persona = serde_json::from_value(json!({})).unwrap();
        assert!(persona.conversation_behavior.pause_on_farewell);
        assert_eq!(persona.reply_delay_factor, ReplyDelayFactor::default());
        assert_eq!(persona.conversation_behavior.skip_reply_tendency, "medium");
        assert_eq!(persona.proactive.silence_after_hours, 8.0);
    }

    #[test]
    fn reply_delay_factor_accepts_number() {
        let persona: Persona =
            serde_json::from_value(json!({ "reply_delay_factor": 0.8 })).unwrap();
        assert_eq!(persona.reply_delay_factor, ReplyDelayFactor { min: 0.8, max: 0.8 });
    }
}
