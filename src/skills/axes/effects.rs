const PVP_CRITICAL_MULTIPLIER: f32 = 1.5;
const PVE_CRITICAL_MULTIPLIER: f32 = 2.0;
const MAX_CRITICAL_LEVEL: u32 = 1000;
const CRITICAL_ROLL_OUTCOMES: u32 = 8_000;
const CRITICAL_OUTCOMES_PER_LEVEL: u32 = 3;

pub(crate) fn multiply_damage(damage: f32, target_is_player: bool, level: u32) -> f32 {
    let critical = roll(level);
    match (critical, target_is_player) {
        (true, true) => damage * PVP_CRITICAL_MULTIPLIER,
        (true, false) => damage * PVE_CRITICAL_MULTIPLIER,
        (false, _) => damage,
    }
}

fn roll(level: u32) -> bool {
    fastrand::u32(..CRITICAL_ROLL_OUTCOMES)
        < level.min(MAX_CRITICAL_LEVEL) * CRITICAL_OUTCOMES_PER_LEVEL
}
