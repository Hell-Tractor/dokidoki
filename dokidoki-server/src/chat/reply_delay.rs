use chrono::{DateTime, Utc};

use crate::{
    config::ReplyDelay,
    domain::Availability,
    utils::{uniform, UnitRng},
};

#[derive(Debug, Clone, PartialEq)]
pub struct ReplyDelayInput {
    pub availability: Availability,
    /// 角色 `persona.reply_delay_factor` 区间；缺省为 1.0–1.0
    pub factor_min: f64,
    pub factor_max: f64,
    pub activity_remaining_secs: Option<f64>,
}

pub fn compute_reply_wait_ms(
    input: &ReplyDelayInput,
    config: &ReplyDelay,
    rng: &mut impl UnitRng,
) -> u64 {
    let base_secs =
        sample_availability_secs(input.availability, input.activity_remaining_secs, config, rng);
    let factor = uniform(input.factor_min, input.factor_max, rng.next_unit());
    let jitter = uniform(config.jitter_min, config.jitter_max, rng.next_unit());
    let mut secs = base_secs * factor * jitter;

    if let Some(cap) = input.activity_remaining_secs {
        secs = secs.min(cap.max(0.0));
    }

    (secs * 1000.0).round().max(0.0) as u64
}

fn sample_availability_secs(
    availability: Availability,
    activity_remaining_secs: Option<f64>,
    config: &ReplyDelay,
    rng: &mut impl UnitRng,
) -> f64 {
    match availability {
        Availability::High => uniform(config.high_min_secs, config.high_max_secs, rng.next_unit()),
        Availability::Low => sample_low_secs(activity_remaining_secs, config, rng),
        Availability::Medium => {
            uniform(config.medium_min_secs, config.medium_max_secs, rng.next_unit())
        }
    }
}

fn sample_low_secs(
    activity_remaining_secs: Option<f64>,
    config: &ReplyDelay,
    rng: &mut impl UnitRng,
) -> f64 {
    let bucket_unit = rng.next_unit();
    let value_unit = rng.next_unit();
    let bucket = (bucket_unit * 100.0).floor() as u32 % 100;
    let short_cut = config.low_short_weight_pct;
    let mid_cut = short_cut.saturating_add(config.low_mid_weight_pct);

    if bucket < short_cut {
        uniform(
            config.low_short_min_secs,
            config.low_short_max_secs,
            value_unit,
        )
    } else if bucket < mid_cut {
        let cap = activity_remaining_secs
            .unwrap_or(config.low_mid_default_remaining_secs)
            .clamp(config.low_mid_cap_min_secs, config.low_mid_cap_max_secs);
        uniform(config.low_mid_min_secs, cap, value_unit)
    } else {
        activity_remaining_secs
            .unwrap_or(config.low_long_default_remaining_secs)
            .clamp(
                config.low_long_clamp_min_secs,
                config.low_long_clamp_max_secs,
            )
    }
}

pub fn activity_remaining_secs(activity_ends_at: Option<DateTime<Utc>>, now: DateTime<Utc>) -> Option<f64> {
    activity_ends_at.map(|end| {
        let remaining = (end - now).num_milliseconds() as f64 / 1000.0;
        remaining.max(0.0)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::ScriptedRng;

    fn production_config() -> ReplyDelay {
        ReplyDelay::production_defaults()
    }

    fn input(
        availability: Availability,
        factor_min: f64,
        factor_max: f64,
        remaining: Option<f64>,
    ) -> ReplyDelayInput {
        ReplyDelayInput {
            availability,
            factor_min,
            factor_max,
            activity_remaining_secs: remaining,
        }
    }

    #[test]
    fn high_availability_reply_is_short() {
        // base=0.0 → 0.3s, factor=0.0 → 1.0, jitter=0.0 → 0.85
        let mut rng = ScriptedRng::new(vec![0.0, 0.0, 0.0]);
        let ms = compute_reply_wait_ms(
            &input(Availability::High, 1.0, 1.0, None),
            &production_config(),
            &mut rng,
        );
        assert!(ms >= 250);
        assert!(ms <= 2_300);
    }

    #[test]
    fn medium_availability_reply_is_longer() {
        let mut rng = ScriptedRng::new(vec![0.0, 0.0, 0.0]);
        let ms = compute_reply_wait_ms(
            &input(Availability::Medium, 1.0, 1.0, None),
            &production_config(),
            &mut rng,
        );
        assert!(ms >= 25_000);
        assert!(ms <= 345_000);
    }

    #[test]
    fn higher_factor_increases_delay() {
        let cfg = production_config();
        let mut fast_rng = ScriptedRng::new(vec![0.5, 0.5, 0.5]);
        let mut slow_rng = ScriptedRng::new(vec![0.5, 0.5, 0.5]);
        let fast = compute_reply_wait_ms(
            &input(Availability::High, 0.5, 0.7, None),
            &cfg,
            &mut fast_rng,
        );
        let slow = compute_reply_wait_ms(
            &input(Availability::High, 1.3, 1.6, None),
            &cfg,
            &mut slow_rng,
        );
        assert!(slow > fast);
    }

    #[test]
    fn caps_delay_by_activity_remaining() {
        let mut rng = ScriptedRng::new(vec![0.9, 0.9, 0.9]);
        let ms = compute_reply_wait_ms(
            &input(Availability::Medium, 1.0, 1.0, Some(10.0)),
            &production_config(),
            &mut rng,
        );
        assert!(ms <= 10_000);
    }

    #[test]
    fn low_bucket_and_value_use_independent_draws() {
        // bucket=0.1 → short; value=0.0 → min (60s); factor/jitter fixed at mid
        let mut low_value = ScriptedRng::new(vec![0.1, 0.0, 0.5, 0.5]);
        let mut high_value = ScriptedRng::new(vec![0.1, 1.0, 0.5, 0.5]);
        let low = compute_reply_wait_ms(
            &input(Availability::Low, 1.0, 1.0, None),
            &production_config(),
            &mut low_value,
        );
        let high = compute_reply_wait_ms(
            &input(Availability::Low, 1.0, 1.0, None),
            &production_config(),
            &mut high_value,
        );
        assert!(high > low);
    }
}
