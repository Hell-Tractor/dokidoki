pub mod api;
pub mod config;
pub mod domain;
pub mod error;
pub mod state;

pub(crate) mod db;

pub mod test_support;

pub use error::Result;

pub async fn run() -> Result<()> {
    use std::sync::Arc;

    let shared_state = Arc::new(state::AppState::new().await?);

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
