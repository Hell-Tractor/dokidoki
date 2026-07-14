use std::sync::Arc;

use crate::{chat::ChatService, config, error::Result, llm::LlmClient, upload::UploadStore, ws_hub::WsHub};

const CONFIG_PATH: &str = "config.toml";

pub struct AppState {
    pub config: config::Config,
    pub db: sqlx::MySqlPool,
    pub upload: UploadStore,
    pub llm: Arc<LlmClient>,
    pub ws_hub: Arc<WsHub>,
    pub chat: Arc<ChatService>,
}

impl AppState {
    pub async fn new() -> Result<Self> {
        tracing::info!("Loading configuration from {}", CONFIG_PATH);
        let config = config::Config::load_from_file(CONFIG_PATH)?;
        tracing::info!("Configuration loaded successfully");

        let db = init_database(&config.database.url).await?;

        Ok(Self::from_parts(config, db))
    }

    pub fn from_parts(config: config::Config, db: sqlx::MySqlPool) -> Self {
        let upload = UploadStore::new(&config.upload.dir);
        if let Err(err) = upload.ensure_dirs().and_then(|_| upload.bootstrap_avatars()) {
            tracing::warn!("upload bootstrap skipped: {err}");
        }

        let llm = Arc::new(LlmClient::from_config(&config.llm));
        let ws_hub = Arc::new(WsHub::new());
        let chat = Arc::new(ChatService::new(
            db.clone(),
            llm.clone(),
            ws_hub.clone(),
            config.chat.clone(),
            config.summary.clone(),
            config.proactive.clone(),
        ));
        Self {
            config,
            db,
            upload,
            llm,
            ws_hub,
            chat,
        }
    }
}

async fn init_database(url: &str) -> Result<sqlx::MySqlPool> {
    tracing::info!("Connecting to database");
    let pool = crate::db::pool::connect(url).await?;
    tracing::info!("Running database migrations");
    sqlx::migrate!("./migrations").run(&pool).await?;
    tracing::info!("Database ready");
    Ok(pool)
}
