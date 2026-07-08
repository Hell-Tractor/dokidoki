use serde::{Deserialize, Serialize};

use crate::error::Result;

#[derive(Serialize, Deserialize)]
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
        todo!()
    }
}

#[derive(Serialize, Deserialize)]
pub struct Server {
    pub host: String,
    pub port: u16,
}

#[derive(Serialize, Deserialize)]
pub struct Auth {
    pub password_cost: u32,
    pub token_prefix: String,
}

#[derive(Serialize, Deserialize)]
pub struct Database {
    pub url: String,
}

#[derive(Serialize, Deserialize)]
pub struct Llm {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub vision_model: String,
}

#[derive(Serialize, Deserialize)]
pub struct Upload {
    pub dir: String,
    pub max_bytes: usize,
    pub allowed_types: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Chat {
    pub burst_silence_ms: u32,
    pub min_reply_delay_ms: u32,
    pub max_reply_delay_ms: u32,
    pub bubble_delay_base_ms: u32,
    pub bubble_delay_per_char_ms: u32,
}

#[derive(Serialize, Deserialize)]
pub struct Summary {
    pub trigger_turns: u32,
    pub keep_recent_turns: u32,
}

#[derive(Serialize, Deserialize)]
pub struct Push {
    pub fcm_credentials_path: String,
}

#[derive(Serialize, Deserialize)]
pub struct Proactive {
    pub default_max_per_day: u32,
}