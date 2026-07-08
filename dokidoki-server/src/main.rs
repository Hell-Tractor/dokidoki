use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, fmt::writer::MakeWriterExt};

mod state;
mod config;
mod error;

#[tokio::main]
async fn main() {
    let _logger_guard = init_logger();

}

fn init_logger() -> WorkerGuard {
    let file_appender = tracing_appender::rolling::daily("logs", "dokidoki.log");
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);

    let subscriber = tracing_subscriber::fmt()
        .with_writer(file_writer.and(std::io::stderr))
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set global subscriber");

    guard
}