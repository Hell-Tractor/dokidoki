pub mod api;
pub mod chat;
pub mod config;
pub mod domain;
pub mod error;
pub mod llm;
pub mod memory;
pub mod prompt;
pub mod schedule;
pub mod state;
pub mod summary;
pub mod time;
pub mod upload;
pub mod utils;
pub mod ws_hub;

pub(crate) mod db;

pub mod test_support;

pub use error::Result;

pub async fn run() -> Result<()> {
    use std::sync::Arc;

    let shared_state = Arc::new(state::AppState::new().await?);

    let scheduler_pool = shared_state.db.clone();
    let memory_pool = shared_state.db.clone();
    tokio::spawn(async move {
        schedule::run_scheduler(scheduler_pool).await;
    });
    tokio::spawn(async move {
        memory::run_expiry_cleanup(memory_pool).await;
    });

    let addr = format!(
        "{}:{}",
        shared_state.config.server.host, shared_state.config.server.port
    );
    tracing::info!("Starting server at {}", addr);
    let app = api::router(shared_state.clone());

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
