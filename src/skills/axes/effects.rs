const PVP_DAMAGE_MULTIPLIER: f32 = 1.5;
const PVE_DAMAGE_MULTIPLIER: f32 = 2.0;

pub(crate) fn multiply_damage(damage: f32, target_is_player: bool) -> f32 {
    let multiplier = if target_is_player {
        PVP_DAMAGE_MULTIPLIER
    } else {
        PVE_DAMAGE_MULTIPLIER
    };
    damage * multiplier
}

#[cfg(test)]
mod tests {
    use super::multiply_damage;

    #[test]
    fn applies_pvp_multiplier_against_players() {
        assert_eq!(multiply_damage(5.0, true), 7.5);
    }

    #[test]
    fn applies_pve_multiplier_against_non_players() {
        assert_eq!(multiply_damage(0.5, false), 1.0);
    }
}
