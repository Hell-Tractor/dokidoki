/// 在 `[0, reply_wait)` 内采样已读延迟；`high` 时接近即时，但仍早于 typing。
pub fn sample_read_receipt_delay_ms(
    availability: &str,
    reply_wait_ms: u64,
    random_unit: f64,
) -> u64 {
    if reply_wait_ms == 0 {
        return 0;
    }

    let upper = reply_wait_ms.saturating_sub(1);
    if upper == 0 {
        return 0;
    }

    match availability {
        "high" => {
            let cap = upper.min(500);
            (random_unit * cap as f64).round() as u64
        }
        _ => (random_unit * upper as f64).round() as u64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_delay_is_before_reply_wait() {
        let reply_wait = 5_000;
        let delay = sample_read_receipt_delay_ms("medium", reply_wait, 0.99);
        assert!(delay < reply_wait);
    }

    #[test]
    fn high_availability_read_is_quick() {
        let delay = sample_read_receipt_delay_ms("high", 2_000, 0.5);
        assert!(delay <= 500);
    }
}
