mod effects;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use pumpkin_plugin_api::common::{BlockPos, Hand};
use pumpkin_plugin_api::events::{
    BlockBreakEvent, BlockDropItemEvent, EventData, EventHandler, EventPriority,
    PlayerLevelChangeEvent,
};
use pumpkin_plugin_api::server::CommandSender;
use pumpkin_plugin_api::{Context, GameRule, GameRuleValue, Result, Server};

use super::skill_manager::player_uuid;
pub(crate) use effects::MiningEffects;

pub(crate) fn register(context: &Context, effects: Arc<MiningEffects>) -> Result<()> {
    let events = MiningEvents {
        effects,
        pending: Arc::default(),
    };
    let priority = EventPriority::Highest;
    context.register_event_handler::<PlayerLevelChangeEvent, _>(events.clone(), priority, true)?;
    context.register_event_handler::<BlockBreakEvent, _>(events.clone(), priority, true)?;
    context.register_event_handler::<BlockDropItemEvent, _>(events, priority, true)?;
    Ok(())
}

type LootKey = (String, i32, i32, i32);
const PENDING_LOOT_LIFETIME: Duration = Duration::from_secs(1);

struct PendingLoot {
    recorded_at: Instant,
    player_uuid: u128,
    tool: String,
    fortune_level: u32,
}

#[derive(Clone)]
struct MiningEvents {
    effects: Arc<MiningEffects>,
    pending: Arc<Mutex<HashMap<LootKey, PendingLoot>>>,
}

impl EventHandler<PlayerLevelChangeEvent> for MiningEvents {
    fn handle(
        &self,
        _server: Server,
        data: EventData<PlayerLevelChangeEvent>,
    ) -> EventData<PlayerLevelChangeEvent> {
        self.effects.apply_speed(&data.player, data.new_level);
        data
    }
}

impl EventHandler<BlockBreakEvent> for MiningEvents {
    fn handle(
        &self,
        _server: Server,
        data: EventData<BlockBreakEvent>,
    ) -> EventData<BlockBreakEvent> {
        if data.cancelled || !data.should_drop || !has_fortune_drops(&data.block) {
            return data;
        }
        let Some(player) = data.player.as_ref() else {
            return data;
        };
        let world = player.get_world();
        if !matches!(
            world.get_game_rule(GameRule::BlockDrops),
            GameRuleValue::Bool(true)
        ) {
            return data;
        }
        // Pumpkin maps Right to the selected hotbar slot.
        let Some(tool) = player.get_item_in_hand(Hand::Right) else {
            return data;
        };
        let player_uuid = player_uuid(player);
        let key = loot_key(world.get_dimension(), data.block_pos);
        let now = Instant::now();
        let replacement =
            self.effects
                .bonus_fortune_level(player_uuid, &tool)
                .map(|fortune_level| PendingLoot {
                    recorded_at: now,
                    player_uuid,
                    tool: tool.get_registry_key(),
                    fortune_level,
                });
        let mut pending = self.pending.lock().expect("pending mining loot lock");
        pending.retain(|_, loot| now.duration_since(loot.recorded_at) < PENDING_LOOT_LIFETIME);
        pending.remove(&key);
        if let Some(replacement) = replacement {
            pending.insert(key, replacement);
        }
        data
    }
}

impl EventHandler<BlockDropItemEvent> for MiningEvents {
    fn handle(
        &self,
        server: Server,
        mut data: EventData<BlockDropItemEvent>,
    ) -> EventData<BlockDropItemEvent> {
        let dimension = data.target_world.get_dimension();
        let replacement = self
            .pending
            .lock()
            .expect("pending mining loot lock")
            .remove(&loot_key(dimension.clone(), data.block_pos));
        let Some(replacement) = replacement else {
            return data;
        };
        if data.cancelled
            || data.items.is_empty()
            || replacement.recorded_at.elapsed() >= PENDING_LOOT_LIFETIME
            || data
                .player
                .as_ref()
                .is_some_and(|player| player_uuid(player) != replacement.player_uuid)
        {
            return data;
        }

        // Pumpkin only applies cancellation to this event. Roll the intact block's
        // loot table with a virtual tool before suppressing the original item drops.
        let BlockPos { x, y, z } = data.block_pos;
        let tool = replacement.tool;
        let level = replacement.fortune_level;
        server.execute_command(
            &format!(
                "execute in {dimension} run loot spawn {x} {y} {z} mine {x} {y} {z} {tool}[minecraft:enchantments={{\"minecraft:fortune\":{level}}}]"
            ),
            CommandSender::Console,
        );
        data.cancelled = true;
        data
    }
}

fn loot_key(dimension: String, BlockPos { x, y, z }: BlockPos) -> LootKey {
    (dimension, x, y, z)
}

fn has_fortune_drops(block: &str) -> bool {
    let block = block.strip_prefix("minecraft:").unwrap_or(block);
    matches!(
        block.strip_prefix("deepslate_").unwrap_or(block),
        "coal_ore"
            | "copper_ore"
            | "iron_ore"
            | "gold_ore"
            | "redstone_ore"
            | "lapis_ore"
            | "diamond_ore"
            | "emerald_ore"
            | "nether_gold_ore"
            | "nether_quartz_ore"
    )
}
