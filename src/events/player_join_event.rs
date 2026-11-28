use std::sync::Arc;

use pumpkin_plugin_api::events::{EventData, EventHandler, EventPriority, PlayerJoinEvent};
use pumpkin_plugin_api::{Context, Result, Server};

use crate::abilities::unarmed::berserk;
use crate::skills::mining::MiningEffects;
use crate::ui::bossbar_manager::BossBarManager;

pub(crate) fn register(
    context: &Context,
    mining_effects: Arc<MiningEffects>,
    boss_bars: Arc<BossBarManager>,
) -> Result<()> {
    context.register_event_handler(
        PlayerJoin {
            mining_effects,
            boss_bars,
        },
        EventPriority::Highest,
        true,
    )?;
    Ok(())
}

struct PlayerJoin {
    mining_effects: Arc<MiningEffects>,
    boss_bars: Arc<BossBarManager>,
}

impl EventHandler<PlayerJoinEvent> for PlayerJoin {
    fn handle(
        &self,
        _server: Server,
        data: EventData<PlayerJoinEvent>,
    ) -> EventData<PlayerJoinEvent> {
        if data.cancelled {
            return data;
        }

        let player = &data.player;
        self.boss_bars.add_player(player);
        self.mining_effects.apply_speed(player);
        berserk::remove_stale_speed_modifier(player);
        data
    }
}
