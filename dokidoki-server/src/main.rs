use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    let _logger_guard = init_logger();

    if let Err(e) = dokidoki_server::run().await {
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
