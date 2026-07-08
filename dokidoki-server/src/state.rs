use crate::{config, error::Result};

const CONFIG_PATH: &str = "config.toml";

pub struct AppState {
    pub config: config::Config,
}

impl AppState {
    pub fn new() -> Result<Self> {
        tracing::info!("Loading configuration from {}", CONFIG_PATH);
        let config = config::Config::load_from_file(CONFIG_PATH)?;
        tracing::info!("Configuration loaded successfully");
        Ok(AppState { config })
    }
}