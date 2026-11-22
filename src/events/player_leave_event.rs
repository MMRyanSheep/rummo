use std::sync::Arc;

use pumpkin_plugin_api::events::{EventData, EventHandler, EventPriority, PlayerLeaveEvent};
use pumpkin_plugin_api::{Context, Result, Server};

use crate::skills::mining::MiningEffects;
use crate::ui::bossbar_manager::BossBarManager;

pub(crate) fn register(
    context: &Context,
    mining_effects: Arc<MiningEffects>,
    boss_bars: Arc<BossBarManager>,
) -> Result<()> {
    context.register_event_handler(
        PlayerLeave {
            mining_effects,
            boss_bars,
        },
        EventPriority::Highest,
        true,
    )?;
    Ok(())
}

struct PlayerLeave {
    mining_effects: Arc<MiningEffects>,
    boss_bars: Arc<BossBarManager>,
}

impl EventHandler<PlayerLeaveEvent> for PlayerLeave {
    fn handle(
        &self,
        _server: Server,
        data: EventData<PlayerLeaveEvent>,
    ) -> EventData<PlayerLeaveEvent> {
        self.boss_bars.remove_player(&data.player);
        self.mining_effects.remove_fortune(&data.player);
        data
    }
}
