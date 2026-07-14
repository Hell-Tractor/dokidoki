use serde::Deserialize;

use crate::error::Result;

#[derive(Deserialize)]
pub struct Config {
    pub server: Server,
    pub auth: Auth,
    pub database: Database,
    pub llm: Llm,
    pub upload: Upload,
    pub chat: Chat,
    pub summary: Summary,
    pub push: Push,
    pub proactive: Proactive,
}

impl Config {
    pub fn load_from_file(path: &str) -> Result<Self> {
        let config_str = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&config_str)?;
        Ok(config)
    }

    /// 集成测试用配置；`password_cost` 取较低值以加快 argon2。
    pub fn for_test(database_url: impl Into<String>) -> Self {
        Self {
            server: Server {
                host: "127.0.0.1".into(),
                port: 0,
            },
            auth: Auth {
                password_cost: 4,
                token_prefix: "doki_".into(),
            },
            database: Database {
                url: database_url.into(),
            },
            llm: Llm {
                mode: "fake".into(),
                fake_default: "[REPLY]".into(),
                base_url: "http://localhost".into(),
                api_key: "test".into(),
                model: "test".into(),
                vision_model: "test".into(),
            },
            upload: Upload {
                dir: "/tmp".into(),
                max_bytes: 1024,
                allowed_types: vec!["image/png".into()],
            },
            chat: Chat {
                burst_silence_ms: 1,
                bubble_delay_base_ms: 1,
                bubble_delay_per_char_ms: 1,
                reply_delay: ReplyDelay::for_test(),
            },
            summary: Summary {
                trigger_turns: 80,
                keep_recent_turns: 40,
                max_summary_chars: 800,
            },
            push: Push {
                fcm_credentials_path: "/tmp/fcm.json".into(),
            },
            proactive: Proactive {
                default_max_per_day: 20,
                availability_high: 0.45,
                availability_medium: 0.25,
                availability_low: 0.05,
            },
        }
    }
}

#[derive(Deserialize)]
pub struct Server {
    pub host: String,
    pub port: u16,
}

#[derive(Deserialize)]
pub struct Auth {
    pub password_cost: u32,
    pub token_prefix: String,
}

#[derive(Deserialize)]
pub struct Database {
    pub url: String,
}

#[derive(Deserialize)]
pub struct Llm {
    pub mode: String,
    pub fake_default: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub vision_model: String,
}

#[derive(Deserialize)]
pub struct Upload {
    pub dir: String,
    pub max_bytes: usize,
    pub allowed_types: Vec<String>,
}

#[derive(Clone, Deserialize)]
pub struct Chat {
    pub burst_silence_ms: u32,
    pub bubble_delay_base_ms: u32,
    pub bubble_delay_per_char_ms: u32,
    pub reply_delay: ReplyDelay,
}

/// M-15 忙碌回复延迟参数（秒 / 系数），对应 `docs/详细设计说明书.md` §8。
#[derive(Clone, Deserialize)]
pub struct ReplyDelay {
    pub high_min_secs: f64,
    pub high_max_secs: f64,
    pub medium_min_secs: f64,
    pub medium_max_secs: f64,
    pub low_short_min_secs: f64,
    pub low_short_max_secs: f64,
    pub low_mid_min_secs: f64,
    pub low_mid_default_remaining_secs: f64,
    pub low_mid_cap_min_secs: f64,
    pub low_mid_cap_max_secs: f64,
    pub low_long_default_remaining_secs: f64,
    pub low_long_clamp_min_secs: f64,
    pub low_long_clamp_max_secs: f64,
    /// low 短延迟桶权重（百分比，如 30 表示 30%）
    pub low_short_weight_pct: u32,
    /// low 中延迟桶权重（百分比）；剩余归入长延迟桶
    pub low_mid_weight_pct: u32,
    pub jitter_min: f64,
    pub jitter_max: f64,
}

impl ReplyDelay {
    pub fn production_defaults() -> Self {
        Self {
            high_min_secs: 0.3,
            high_max_secs: 2.0,
            medium_min_secs: 30.0,
            medium_max_secs: 300.0,
            low_short_min_secs: 60.0,
            low_short_max_secs: 300.0,
            low_mid_min_secs: 300.0,
            low_mid_default_remaining_secs: 600.0,
            low_mid_cap_min_secs: 300.0,
            low_mid_cap_max_secs: 3600.0,
            low_long_default_remaining_secs: 300.0,
            low_long_clamp_min_secs: 60.0,
            low_long_clamp_max_secs: 3600.0,
            low_short_weight_pct: 30,
            low_mid_weight_pct: 45,
            jitter_min: 0.85,
            jitter_max: 1.15,
        }
    }

    pub fn for_test() -> Self {
        Self {
            high_min_secs: 0.0,
            high_max_secs: 0.0,
            medium_min_secs: 0.0,
            medium_max_secs: 0.0,
            low_short_min_secs: 0.0,
            low_short_max_secs: 0.0,
            low_mid_min_secs: 0.0,
            low_mid_default_remaining_secs: 0.0,
            low_mid_cap_min_secs: 0.0,
            low_mid_cap_max_secs: 0.0,
            low_long_default_remaining_secs: 0.0,
            low_long_clamp_min_secs: 0.0,
            low_long_clamp_max_secs: 0.0,
            low_short_weight_pct: 30,
            low_mid_weight_pct: 45,
            jitter_min: 1.0,
            jitter_max: 1.0,
        }
    }
}

#[derive(Clone, Deserialize)]
pub struct Summary {
    pub trigger_turns: u32,
    pub keep_recent_turns: u32,
    pub max_summary_chars: u32,
}

#[derive(Deserialize)]
pub struct Push {
    pub fcm_credentials_path: String,
}

#[derive(Clone, Deserialize)]
pub struct Proactive {
    pub default_max_per_day: u32,
    /// availability=high 时基础触发概率（再乘 persona `probability_factor`）
    pub availability_high: f64,
    /// availability=medium
    pub availability_medium: f64,
    /// availability=low
    pub availability_low: f64,
}

impl Proactive {
    pub fn base_probability(&self, availability: &str) -> f64 {
        match availability {
            "high" => self.availability_high,
            "low" => self.availability_low,
            _ => self.availability_medium,
        }
    }
}
