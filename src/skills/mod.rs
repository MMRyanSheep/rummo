pub(crate) mod axes;
mod experience;
pub(crate) mod mining;
pub mod skill_manager;
pub(crate) mod unarmed;

/// Combat experience for one hit: one point per whole point of damage.
pub(crate) fn experience_for_damage(damage: f32) -> u32 {
    if damage.is_finite() {
        damage.floor() as u32
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounds_damage_down_to_whole_experience() {
        for (damage, experience) in [
            (0.9, 0),
            (1.0, 1),
            (5.9, 5),
            (-1.5, 0),
            (f32::MAX, u32::MAX),
        ] {
            assert_eq!(experience_for_damage(damage), experience);
        }
        assert_eq!(experience_for_damage(f32::NAN), 0);
        assert_eq!(experience_for_damage(f32::INFINITY), 0);
    }
}
