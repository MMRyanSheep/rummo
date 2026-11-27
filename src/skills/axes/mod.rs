pub(crate) mod armor;
pub(crate) mod effects;

use pumpkin_plugin_api::Item;

pub(crate) fn experience_for_damage(damage: f32) -> u32 {
    if damage.is_finite() {
        damage.floor() as u32
    } else {
        0
    }
}

pub(crate) fn is_axe(item: Item) -> bool {
    matches!(
        item,
        Item::WoodenAxe
            | Item::StoneAxe
            | Item::CopperAxe
            | Item::IronAxe
            | Item::GoldenAxe
            | Item::DiamondAxe
            | Item::NetheriteAxe
    )
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

    #[test]
    fn recognizes_all_vanilla_axes() {
        for item in [
            Item::WoodenAxe,
            Item::StoneAxe,
            Item::CopperAxe,
            Item::IronAxe,
            Item::GoldenAxe,
            Item::DiamondAxe,
            Item::NetheriteAxe,
        ] {
            assert!(is_axe(item));
        }
        assert!(!is_axe(Item::DiamondPickaxe));
        assert!(!is_axe(Item::DiamondSword));
    }
}
