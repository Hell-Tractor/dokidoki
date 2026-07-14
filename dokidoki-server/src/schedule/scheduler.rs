use std::time::Duration;

use sqlx::MySqlPool;

use crate::utils::gen_u64_inclusive;

use super::refresh_all_character_states;

const MIN_TICK_SECS: u64 = 30;
const MAX_TICK_SECS: u64 = 90;

pub fn next_tick_delay() -> Duration {
    Duration::from_secs(gen_u64_inclusive(MIN_TICK_SECS, MAX_TICK_SECS))
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
