mod effects;

use std::sync::Arc;

use pumpkin_plugin_api::events::{EventData, EventHandler, EventPriority, PlayerLevelChangeEvent};
use pumpkin_plugin_api::{Context, Result, Server};

pub(crate) use effects::MiningEffects;

pub(crate) fn register(context: &Context, effects: Arc<MiningEffects>) -> Result<()> {
    context.register_event_handler(MiningEvents { effects }, EventPriority::Highest, true)?;
    Ok(())
}

struct MiningEvents {
    effects: Arc<MiningEffects>,
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
