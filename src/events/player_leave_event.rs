use std::sync::Arc;

use pumpkin_plugin_api::events::{EventData, EventHandler, EventPriority, PlayerLeaveEvent};
use pumpkin_plugin_api::{Context, Result, Server};

use crate::skills::mining::MiningEffects;

pub(crate) fn register(context: &Context, mining_effects: Arc<MiningEffects>) -> Result<()> {
    context.register_event_handler(PlayerLeave { mining_effects }, EventPriority::Highest, true)?;
    Ok(())
}

struct PlayerLeave {
    mining_effects: Arc<MiningEffects>,
}

impl EventHandler<PlayerLeaveEvent> for PlayerLeave {
    fn handle(
        &self,
        _server: Server,
        data: EventData<PlayerLeaveEvent>,
    ) -> EventData<PlayerLeaveEvent> {
        self.mining_effects.remove_fortune(&data.player);
        data
    }
}
