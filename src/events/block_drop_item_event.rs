use pumpkin_plugin_api::events::{BlockDropItemEvent, EventData, EventHandler, EventPriority};
use pumpkin_plugin_api::{Context, ItemStack, Result, Server};

use crate::skills::mining::drop_multiplier_service::{self, DropMultiplier};
use crate::skills::skill_manager::SkillManager;

// Pumpkin 0.2.2 needs patches/pumpkin-block-drop-item.patch to pass the player and apply item edits.
pub(crate) fn register(context: &Context) -> Result<()> {
    context.register_event_handler(BlockDropItems, EventPriority::Highest, true)?;
    Ok(())
}

struct BlockDropItems;

impl EventHandler<BlockDropItemEvent> for BlockDropItems {
    fn handle(
        &self,
        _server: Server,
        mut data: EventData<BlockDropItemEvent>,
    ) -> EventData<BlockDropItemEvent> {
        if data.cancelled || data.items.is_empty() {
            return data;
        }
        let Some(player) = data.player.as_ref() else {
            return data;
        };
        let probabilities = match SkillManager::mining_drop_probabilities(player) {
            Ok(probabilities) => probabilities,
            Err(error) => {
                tracing::error!(%error, "Failed to calculate mining drop probabilities");
                return data;
            }
        };
        let multiplier = drop_multiplier_service::roll(probabilities);
        if multiplier != DropMultiplier::Normal {
            multiply_stacks(&mut data.items, multiplier);
        }
        data
    }
}

fn multiply_stacks(items: &mut Vec<ItemStack>, multiplier: DropMultiplier) {
    let mut overflow = Vec::new();
    for stack in items.iter() {
        let total = u16::from(stack.get_count()) * multiplier as u16;
        if total == 0 {
            continue;
        }

        let limit = u16::from(stack.get_max_count().max(1));
        let first = total.min(limit);
        stack.set_count(first as u8);
        if total > first {
            let registry_key = stack.get_registry_key();
            let components = stack.get_components();
            let mut remaining = total - first;
            while remaining > 0 {
                let count = remaining.min(limit);
                let extra = ItemStack::new(&registry_key, count as u8);
                for component in &components {
                    extra.set_component(component.component, &component.value);
                }
                overflow.push(extra);
                remaining -= count;
            }
        }
    }
    items.extend(overflow);
}
