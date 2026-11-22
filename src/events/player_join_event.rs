use std::sync::Arc;

use pumpkin_plugin_api::events::{EventData, EventHandler, EventPriority, PlayerJoinEvent};
use pumpkin_plugin_api::{Context, Result, Server};

use crate::skills::{mining::MiningEffects, skill_manager::SkillManager};
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
        self.mining_effects
            .apply_speed(player, player.get_experience_level());
        if let Err(error) = SkillManager::refresh_mining_fortune(player) {
            tracing::error!(%error, "Failed to apply mining fortune on join");
        }
        data
    }
}
