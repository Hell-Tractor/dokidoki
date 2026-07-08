use std::sync::Arc;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, fmt::writer::MakeWriterExt};

use crate::error::Result;

mod state;
mod config;
mod error;
mod api;

async fn run() -> Result<()> {
    let shared_state = Arc::new(state::AppState::new()?);

    let addr = format!("{}:{}", shared_state.config.server.host, shared_state.config.server.port);
    tracing::info!("Starting server at {}", addr);
    let app = api::router(shared_state.clone());

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[tokio::main]
async fn main() {
    let _logger_guard = init_logger();

    if let Err(e) = run().await {
        tracing::error!("Application error: {:?}", e);
    }
}

fn init_logger() -> WorkerGuard {
    let file_appender = tracing_appender::rolling::daily("logs", "dokidoki.log");
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);

    let subscriber = tracing_subscriber::fmt()
        .with_writer(file_writer.and(std::io::stderr))
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set global subscriber");
    tracing::info!("Logger initialized");

    guard
}