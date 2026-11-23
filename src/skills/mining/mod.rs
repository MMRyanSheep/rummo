pub(crate) mod drop_multiplier_service;
mod effects;

use std::sync::Arc;

use pumpkin_plugin_api::Result;

use super::skill_manager::{SkillManager, SkillType};

pub(crate) use effects::MiningEffects;

pub(crate) fn register(effects: Arc<MiningEffects>) -> Result<()> {
    SkillManager::set_level_change_handler(Arc::new(move |player, skill| {
        if skill == SkillType::Mining {
            effects.apply_speed(player);
        }
    }))
}
