use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq)]
pub struct ReplyDelayInput {
    pub availability: String,
    pub proactive_tendency: String,
    pub activity_remaining_secs: Option<f64>,
}

pub fn compute_reply_wait_ms(input: &ReplyDelayInput, random_unit: f64) -> u64 {
    let base_secs = sample_availability_secs(&input.availability, input.activity_remaining_secs, random_unit);
    let factor = personality_factor(&input.proactive_tendency, random_unit);
    let jitter = jitter_factor(random_unit);
    let mut secs = base_secs * factor * jitter;

    if let Some(cap) = input.activity_remaining_secs {
        secs = secs.min(cap.max(0.0));
    }

    (secs * 1000.0).round().max(0.0) as u64
}

fn sample_availability_secs(
    availability: &str,
    activity_remaining_secs: Option<f64>,
    random_unit: f64,
) -> f64 {
    match availability {
        "high" => uniform(0.3, 2.0, random_unit),
        "low" => sample_low_secs(activity_remaining_secs, random_unit),
        _ => uniform(30.0, 300.0, random_unit),
    }
}

fn sample_low_secs(activity_remaining_secs: Option<f64>, random_unit: f64) -> f64 {
    let bucket = (random_unit * 100.0).floor() as u32 % 100;
    if bucket < 30 {
        uniform(60.0, 300.0, random_unit)
    } else if bucket < 75 {
        let cap = activity_remaining_secs.unwrap_or(600.0).clamp(300.0, 3600.0);
        uniform(300.0, cap, random_unit)
    } else {
        activity_remaining_secs.unwrap_or(300.0).clamp(60.0, 3600.0)
    }
}

fn personality_factor(tendency: &str, random_unit: f64) -> f64 {
    match tendency {
        "clingy" => uniform(0.5, 0.7, random_unit),
        "distant" => uniform(1.3, 1.6, random_unit),
        _ => 1.0,
    }
}

fn jitter_factor(random_unit: f64) -> f64 {
    uniform(0.85, 1.15, random_unit)
}

fn uniform(min: f64, max: f64, random_unit: f64) -> f64 {
    if min >= max {
        return min;
    }
    min + random_unit * (max - min)
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

    fn input(availability: &str, tendency: &str, remaining: Option<f64>) -> ReplyDelayInput {
        ReplyDelayInput {
            availability: availability.into(),
            proactive_tendency: tendency.into(),
            activity_remaining_secs: remaining,
        }
    }

    #[test]
    fn high_availability_reply_is_short() {
        let ms = compute_reply_wait_ms(&input("high", "normal", None), 0.0);
        // 0.3s base × 1.0 factor × 0.85 jitter
        assert!(ms >= 250);
        assert!(ms <= 2_300);
    }

    #[test]
    fn medium_availability_reply_is_longer() {
        let ms = compute_reply_wait_ms(&input("medium", "normal", None), 0.0);
        // 30s base × 1.0 factor × 0.85 jitter
        assert!(ms >= 25_000);
        assert!(ms <= 345_000);
    }

    #[test]
    fn distant_personality_increases_delay() {
        let normal = compute_reply_wait_ms(&input("high", "normal", None), 0.5);
        let distant = compute_reply_wait_ms(&input("high", "distant", None), 0.5);
        assert!(distant > normal);
    }

    #[test]
    fn caps_delay_by_activity_remaining() {
        let ms = compute_reply_wait_ms(&input("medium", "normal", Some(10.0)), 0.9);
        assert!(ms <= 10_000);
    }
}
