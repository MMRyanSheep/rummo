use pumpkin_plugin_api::Player;
use serde::Deserialize;

use super::ability_registry::AbilityRegistry;
use super::cooldown_manager::COOLDOWNS;
use crate::skills::skill_manager::player_uuid;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AbilityId {
    SuperBreaker,
}

pub(crate) fn activate(registry: &AbilityRegistry, ability_id: AbilityId, player: &Player) {
    if registry.get(ability_id).is_none() {
        return;
    }
    let player_id = player_uuid(player);
    let is_ready = COOLDOWNS
        .lock()
        .expect("cooldown manager lock")
        .is_ready(player_id, ability_id);
    if !is_ready {
        return;
    }

    match ability_id {
        AbilityId::SuperBreaker => activate_super_breaker(player),
    }
}

fn activate_super_breaker(player: &Player) {
    return;
}
