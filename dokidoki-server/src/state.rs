use crate::{config, error::Result};

const CONFIG_PATH: &str = "config.toml";

pub struct AppState {
    pub config: config::Config,
    pub db: sqlx::MySqlPool,
}

impl AppState {
    pub async fn new() -> Result<Self> {
        tracing::info!("Loading configuration from {}", CONFIG_PATH);
        let config = config::Config::load_from_file(CONFIG_PATH)?;
        tracing::info!("Configuration loaded successfully");

        let db = init_database(&config.database.url).await?;

        Ok(AppState { config, db })
    }

    pub fn from_parts(config: config::Config, db: sqlx::MySqlPool) -> Self {
        Self { config, db }
    }
}

async fn init_database(url: &str) -> Result<sqlx::MySqlPool> {
    tracing::info!("Connecting to database");
    let pool = sqlx::MySqlPool::connect(url).await?;
    tracing::info!("Running database migrations");
    sqlx::migrate!("./migrations").run(&pool).await?;
    tracing::info!("Database ready");
    Ok(pool)
}
