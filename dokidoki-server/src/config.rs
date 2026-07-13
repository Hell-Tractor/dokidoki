use serde::{Deserialize};

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
                min_reply_delay_ms: 1,
                max_reply_delay_ms: 1,
                bubble_delay_base_ms: 1,
                bubble_delay_per_char_ms: 1,
            },
            summary: Summary {
                trigger_turns: 1,
                keep_recent_turns: 1,
            },
            push: Push {
                fcm_credentials_path: "/tmp/fcm.json".into(),
            },
            proactive: Proactive {
                default_max_per_day: 20,
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
    pub min_reply_delay_ms: u32,
    pub max_reply_delay_ms: u32,
    pub bubble_delay_base_ms: u32,
    pub bubble_delay_per_char_ms: u32,
}

#[derive(Deserialize)]
pub struct Summary {
    pub trigger_turns: u32,
    pub keep_recent_turns: u32,
}

#[derive(Deserialize)]
pub struct Push {
    pub fcm_credentials_path: String,
}

#[derive(Deserialize)]
pub struct Proactive {
    pub default_max_per_day: u32,
}