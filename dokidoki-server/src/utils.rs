use rand_core::{OsRng, RngCore};

/// `[0, 1)` 均匀随机源；生产用 OS，单测可注入固定序列。
pub trait UnitRng {
    fn next_unit(&mut self) -> f64;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct OsUnitRng;

impl UnitRng for OsUnitRng {
    fn next_unit(&mut self) -> f64 {
        (OsRng.next_u32() as f64) / (f64::from(u32::MAX) + 1.0)
    }
}

/// 将 `[0, 1)` 的 `unit` 映射到 `[min, max]`。
pub fn uniform(min: f64, max: f64, unit: f64) -> f64 {
    if min >= max {
        return min;
    }
    min + unit * (max - min)
}

/// 在闭区间 `[min, max]` 上均匀抽取 `u64`。
pub fn gen_u64_inclusive(min: u64, max: u64) -> u64 {
    debug_assert!(min <= max);
    let span = max - min + 1;
    min + (OsRng.next_u32() as u64 % span)
}

/// 单测用：按序返回预设 `[0, 1)` 值，耗尽后恒为 `0.0`。
#[derive(Debug, Clone)]
pub struct ScriptedRng {
    values: Vec<f64>,
    index: usize,
}

impl ScriptedRng {
    pub fn new(values: impl Into<Vec<f64>>) -> Self {
        Self {
            values: values.into(),
            index: 0,
        }
    }
}

impl UnitRng for ScriptedRng {
    fn next_unit(&mut self) -> f64 {
        let value = self.values.get(self.index).copied().unwrap_or(0.0);
        self.index += 1;
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_maps_unit_to_range() {
        assert_eq!(uniform(10.0, 20.0, 0.0), 10.0);
        assert_eq!(uniform(10.0, 20.0, 1.0), 20.0);
        assert_eq!(uniform(10.0, 20.0, 0.5), 15.0);
    }

    #[test]
    fn scripted_rng_returns_values_in_order() {
        let mut rng = ScriptedRng::new(vec![0.1, 0.2]);
        assert_eq!(rng.next_unit(), 0.1);
        assert_eq!(rng.next_unit(), 0.2);
        assert_eq!(rng.next_unit(), 0.0);
    }

    #[test]
    fn gen_u64_inclusive_stays_in_bounds() {
        for _ in 0..200 {
            let value = gen_u64_inclusive(30, 90);
            assert!((30..=90).contains(&value));
        }
    }
}
