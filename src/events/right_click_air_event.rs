use pumpkin_plugin_api::common::Hand;
use pumpkin_plugin_api::events::{
    EventData, EventHandler, EventPriority, InteractAction, PlayerInteractEvent,
};
use pumpkin_plugin_api::{Context, IntoItemKey, Item, Player, Result, Server};

use crate::abilities::ability_manager::AbilityId;
use crate::abilities::ability_registry::AbilityRegistry;
use crate::abilities::active_ability_manager;
use crate::skills::axes::is_axe;

pub(crate) fn register(context: &Context, ability_registry: AbilityRegistry) -> Result<()> {
    context.register_event_handler(
        RightClickAir { ability_registry },
        EventPriority::Normal,
        true,
    )?;
    Ok(())
}
struct RightClickAir {
    ability_registry: AbilityRegistry,
}

impl EventHandler<PlayerInteractEvent> for RightClickAir {
    fn handle(
        &self,
        _server: Server,
        mut data: EventData<PlayerInteractEvent>,
    ) -> EventData<PlayerInteractEvent> {   
        if data.cancelled || data.action != InteractAction::RightClickAir {
            return data;
        }
        let (player, held_item) = get_held_item(data.player);
        data.player = player;
        // No stack means an empty main hand.
        if let Some(ability_id) =
            held_item.map_or(Some(AbilityId::Berserk), |item| item.ability_id())
        {
            active_ability_manager::activate(&self.ability_registry, ability_id, &data.player);
        }
        data
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HeldItemCategory {
    Mining,
    Axes,
    Unarmed,
    Other,
}

pub(crate) struct HeldItem {
    #[cfg_attr(not(test), expect(dead_code, reason = "What is this for?"))]
    pub(crate) item_id: String,
    pub(crate) category: HeldItemCategory,
}

impl HeldItem {
    fn ability_id(&self) -> Option<AbilityId> {
        match self.category {
            HeldItemCategory::Mining => Some(AbilityId::SuperBreaker),
            HeldItemCategory::Unarmed => Some(AbilityId::Berserk),
            HeldItemCategory::Axes | HeldItemCategory::Other => None,
        }
    }

    fn from_registry_key(key: String) -> Self {
        let item = Item::from_registry_key(&key);
        let category = match item {
            Some(Item::Air) => HeldItemCategory::Unarmed,
            Some(item) if is_axe(item) => HeldItemCategory::Axes,
            Some(
                Item::WoodenPickaxe
                | Item::StonePickaxe
                | Item::CopperPickaxe
                | Item::IronPickaxe
                | Item::GoldenPickaxe
                | Item::DiamondPickaxe
                | Item::NetheritePickaxe,
            ) => HeldItemCategory::Mining,
            _ => HeldItemCategory::Other,
        };
        Self {
            item_id: key.into_item_key(),
            category,
        }
    }
}

/// Returns the player with their current main-hand item, if any.
pub(crate) fn get_held_item(player: Player) -> (Player, Option<HeldItem>) {
    let held_item = player
        .get_item_in_hand(Hand::Right)
        .map(|stack| HeldItem::from_registry_key(stack.get_registry_key()));
    (player, held_item)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_all_vanilla_pickaxes_to_super_breaker() {
        for item in [
            Item::WoodenPickaxe,
            Item::StonePickaxe,
            Item::CopperPickaxe,
            Item::IronPickaxe,
            Item::GoldenPickaxe,
            Item::DiamondPickaxe,
            Item::NetheritePickaxe,
        ] {
            let held = HeldItem::from_registry_key(item.into_item_key());
            assert_eq!(held.ability_id(), Some(AbilityId::SuperBreaker));
        }

        for item in [Item::IronAxe, Item::Stone] {
            let held = HeldItem::from_registry_key(item.into_item_key());
            assert_eq!(held.ability_id(), None);
        }
    }

    #[test]
    fn maps_only_air_to_berserk() {
        let held = HeldItem::from_registry_key(Item::Air.into_item_key());
        assert_eq!(held.ability_id(), Some(AbilityId::Berserk));
        let held = HeldItem::from_registry_key("custom:glove".into());
        assert_eq!(held.ability_id(), None);
    }

    #[test]
    fn classifies_held_items_and_preserves_their_ids() {
        for (key, id, category) in [
            (
                "diamond_pickaxe",
                "minecraft:diamond_pickaxe",
                HeldItemCategory::Mining,
            ),
            (
                "minecraft:copper_pickaxe",
                "minecraft:copper_pickaxe",
                HeldItemCategory::Mining,
            ),
            (
                "minecraft:iron_axe",
                "minecraft:iron_axe",
                HeldItemCategory::Axes,
            ),
            ("stone", "minecraft:stone", HeldItemCategory::Other),
            (
                "custom:iron_axe",
                "custom:iron_axe",
                HeldItemCategory::Other,
            ),
        ] {
            let held = HeldItem::from_registry_key(key.into());
            assert_eq!(held.item_id, id);
            assert_eq!(held.category, category);
        }
    }
}
