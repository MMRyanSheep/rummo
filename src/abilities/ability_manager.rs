use pumpkin_plugin_api::Player;

use super::cooldown_manager::COOLDOWNS;
use crate::skills::skill_manager::player_uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum AbilityId {
    SuperBreaker,
}

pub(crate) fn activate(ability_id: AbilityId, player: &Player, item_id: &str) {
    let player_id = player_uuid(player);
    let is_ready = COOLDOWNS
        .lock()
        .expect("cooldown manager lock")
        .is_ready(player_id, ability_id);
    if !is_ready {
        return;
    }

    match ability_id {
        AbilityId::SuperBreaker => activate_super_breaker(player, item_id),
    }
}

fn activate_super_breaker(player: &Player, item_id: &str) {
    tracing::trace!(player = %player.get_name(), item_id, "SuperBreaker requested");
}
