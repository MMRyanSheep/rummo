pub(crate) mod armor;
pub(crate) mod effects;

use pumpkin_plugin_api::Item;

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
