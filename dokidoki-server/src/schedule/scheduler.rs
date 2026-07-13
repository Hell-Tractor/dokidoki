use std::time::Duration;

use rand_core::{OsRng, RngCore};
use sqlx::MySqlPool;

use super::refresh_all_character_states;

const MIN_TICK_SECS: u64 = 30;
const MAX_TICK_SECS: u64 = 90;

pub fn next_tick_delay() -> Duration {
    let span = MAX_TICK_SECS - MIN_TICK_SECS + 1;
    let secs = MIN_TICK_SECS + (OsRng.next_u32() as u64 % span);
    Duration::from_secs(secs)
}

/// 后台循环刷新所有角色日程状态；每次 tick 间隔在 30～90s 间均匀随机。
pub async fn run(pool: MySqlPool) {
    tracing::info!("schedule scheduler started (tick interval {MIN_TICK_SECS}s–{MAX_TICK_SECS}s)");

    loop {
        if let Err(err) = refresh_all_character_states(&pool).await {
            tracing::warn!("schedule tick failed: {err}");
        }
        let delay = next_tick_delay();
        tracing::debug!(?delay, "schedule next tick");
        tokio::time::sleep(delay).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_tick_delay_within_bounds() {
        for _ in 0..200 {
            let delay = next_tick_delay();
            assert!(delay >= Duration::from_secs(MIN_TICK_SECS));
            assert!(delay <= Duration::from_secs(MAX_TICK_SECS));
        }
    }
}
