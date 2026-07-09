use std::sync::Arc;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use crate::error::Result;

mod state;
mod config;
mod error;
mod api;
mod auth;

async fn run() -> Result<()> {
    let shared_state = Arc::new(state::AppState::new().await?);

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

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(file_writer)
        .with_ansi(false)
        .with_target(true);

    let terminal_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .with_ansi(true)
        .with_target(true);

    tracing_subscriber::Registry::default()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(file_layer)
        .with(terminal_layer)
        .init();

    tracing::info!("Logger initialized");

    guard
}