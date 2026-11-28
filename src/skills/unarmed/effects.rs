const BASE_BONUS_DAMAGE: f32 = 1.0;
const BONUS_DAMAGE_PER_STEP: f32 = 0.5;
const LEVELS_PER_STEP: u32 = 50;

/// Adds the unarmed bonus, which grows every 50 levels without a cap.
pub(crate) fn add_bonus_damage(damage: f32, level: u32) -> f32 {
    let steps = level / LEVELS_PER_STEP;
    damage + BASE_BONUS_DAMAGE + steps as f32 * BONUS_DAMAGE_PER_STEP
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_half_a_point_every_fifty_levels() {
        for (level, damage) in [
            (1, 2.0),
            (49, 2.0),
            (50, 2.5),
            (99, 2.5),
            (100, 3.0),
            (2000, 22.0),
        ] {
            assert_eq!(add_bonus_damage(1.0, level), damage);
        }
    }
}
