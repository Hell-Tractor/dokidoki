use std::time::Duration;

use sqlx::MySqlPool;

use crate::error::AppError;

pub async fn run_expiry_cleanup(pool: MySqlPool) {
    tracing::info!("memory expiry cleanup started (interval 24h)");
    loop {
        match purge_expired(&pool).await {
            Ok(count) if count > 0 => tracing::info!(count, "purged expired memories"),
            Ok(_) => tracing::debug!("no expired memories to purge"),
            Err(err) => tracing::warn!("memory expiry cleanup failed: {err}"),
        }
        tokio::time::sleep(Duration::from_secs(24 * 60 * 60)).await;
    }
}

async fn purge_expired(pool: &MySqlPool) -> Result<u64, AppError> {
    super::service::purge_expired(pool).await
}
