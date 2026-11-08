//! Shared Hunt Atlas pacing decomposition.
//!
//! Kept dependency-free so the offline hunt-pacing validator and the normal
//! forecast path evaluate the exact same bicycle/dead-time model.

pub const WALKING_PORTION: f64 = 0.83;
pub const WALKING_FLOOR_S: f64 = 1.41;
pub const COMBAT_INTERACTION_TAU_S: f64 = 2.8;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeadTimeParts {
    pub fixed_s: f64,
    pub walking_s: f64,
}

impl DeadTimeParts {
    pub fn total_s(self) -> f64 {
        self.fixed_s + self.walking_s
    }
}

pub fn normalized_bicycle_factor(factor: f64) -> f64 {
    if factor.is_finite() && factor > 1.0 { factor } else { 1.0 }
}

/// /hunt-compatible decomposition requested for Moth:
///
/// walk0 = max(0, 0.83*sMap - 1.41)
/// fixed = sMap - walk0
///
/// The 1.41 s floor belongs to fixed time. Only walk0 is speeded by bicycles.
pub fn baseline_dead_time(map_baseline_s: f64) -> DeadTimeParts {
    let baseline = map_baseline_s.max(0.0);
    let walking = (WALKING_PORTION * baseline - WALKING_FLOOR_S).max(0.0);
    DeadTimeParts {
        fixed_s: (baseline - walking).max(0.0),
        walking_s: walking,
    }
}

/// Combat lets wild Pokémon approach while the player is occupied.
pub fn remaining_walking_s(on_foot_walking_s: f64, combat_s: f64) -> f64 {
    let interaction = if combat_s.is_finite() {
        (-combat_s.max(0.0) / COMBAT_INTERACTION_TAU_S).exp()
    } else {
        0.0
    };
    on_foot_walking_s.max(0.0) * interaction
}

pub fn dead_time_parts(
    map_baseline_s: f64,
    combat_s: f64,
    bicycle_factor: f64,
) -> DeadTimeParts {
    let baseline = baseline_dead_time(map_baseline_s);
    DeadTimeParts {
        fixed_s: baseline.fixed_s,
        walking_s: remaining_walking_s(baseline.walking_s, combat_s)
            / normalized_bicycle_factor(bicycle_factor),
    }
}

pub fn dead_time_s(map_baseline_s: f64, combat_s: f64, bicycle_factor: f64) -> f64 {
    dead_time_parts(map_baseline_s, combat_s, bicycle_factor).total_s()
}

pub fn total_cycle_s(map_baseline_s: f64, combat_s: f64, bicycle_factor: f64) -> f64 {
    combat_s.max(0.0) + dead_time_s(map_baseline_s, combat_s, bicycle_factor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_matches_documented_formula() {
        let s = 3.422;
        let parts = baseline_dead_time(s);
        let walking = (0.83 * s - 1.41).max(0.0);
        assert!((parts.walking_s - walking).abs() < 1e-12);
        assert!((parts.fixed_s - (s - walking)).abs() < 1e-12);
        assert!((parts.total_s() - s).abs() < 1e-12);
    }

    #[test]
    fn bicycle_only_divides_reducible_walking() {
        let s = 3.422;
        let foot = dead_time_parts(s, 0.0, 1.0);
        let bike = dead_time_parts(s, 0.0, 2.0);
        assert_eq!(foot.fixed_s, bike.fixed_s);
        assert!((bike.walking_s - foot.walking_s / 2.0).abs() < 1e-12);
    }

    #[test]
    fn long_combat_eliminates_reducible_walk_but_not_fixed_floor() {
        let s = 3.422;
        let parts = dead_time_parts(s, f64::INFINITY, 2.0);
        assert!(parts.walking_s.abs() < 1e-12);
        assert!(parts.fixed_s >= WALKING_FLOOR_S);
    }
}
