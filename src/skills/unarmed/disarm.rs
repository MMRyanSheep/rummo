use pumpkin_plugin_api::Player;
use pumpkin_plugin_api::common::Hand;

const MIN_LEVEL: u32 = 1000;
const DISARM_CHANCE: f64 = 0.33;
/// Inventory slots outside the hotbar.
const STORAGE_SLOTS: std::ops::Range<u32> = 9..36;

/// Rolls to move the target's main-hand item into their storage; a full storage keeps it in hand.
pub(crate) fn try_disarm(target: &Player, level: u32) {
    if !should_disarm(level, fastrand::f64()) {
        return;
    }
    let inventory = target.get_inventory();
    let Some(stack) = inventory.get_item_in_hand(Hand::Right) else {
        return;
    };
    let storage = inventory.as_inventory();
    let Some(slot) = STORAGE_SLOTS
        .into_iter()
        .find(|&slot| storage.get_item(slot).is_none())
    else {
        return;
    };
    // Moving the whole stack keeps its count and components.
    storage.set_item(slot, Some(stack));
    inventory.set_item_in_hand(Hand::Right, None);
}

fn should_disarm(level: u32, roll: f64) -> bool {
    level >= MIN_LEVEL && roll < DISARM_CHANCE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disarms_only_from_level_1000_within_the_chance() {
        assert!(!should_disarm(999, 0.0));
        assert!(should_disarm(1000, 0.0));
        assert!(should_disarm(u32::MAX, 0.329));
        assert!(!should_disarm(1000, 0.33));
    }
}
