use sqlx::mysql::MySqlPoolOptions;
use sqlx::MySqlPool;

use crate::error::Result;

/// 建立 MySQL 连接池，并在每条连接上固定 `time_zone = UTC`。
pub async fn connect(url: &str) -> Result<MySqlPool> {
    MySqlPoolOptions::new()
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                sqlx::query("SET time_zone = '+00:00'")
                    .execute(conn)
                    .await?;
                Ok(())
            })
        })
        .connect(url)
        .await
        .map_err(Into::into)
}
