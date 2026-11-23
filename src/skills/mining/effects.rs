use std::path::Path;

use pumpkin_plugin_api::{Attribute, AttributeModifier, ModifierOperation, Player, Result};
use serde::Deserialize;

use crate::config;

const DEFAULT_CONFIG: &str = include_str!("../../../config/mining.toml");
const SPEED_MODIFIER_ID: &str = "rummo:mining_speed";

#[derive(Deserialize)]
#[serde(default)]
struct MiningConfig {
    speed_bonus_per_level: f64,
}

impl Default for MiningConfig {
    fn default() -> Self {
        Self {
            speed_bonus_per_level: 0.1,
        }
    }
}

pub(crate) struct MiningEffects {
    config: MiningConfig,
}

impl MiningEffects {
    pub(crate) fn load(path: &Path) -> Result<Self> {
        let config: MiningConfig = config::load(path, DEFAULT_CONFIG)?;
        if !config.speed_bonus_per_level.is_finite() || config.speed_bonus_per_level < 0.0 {
            return Err(format!(
                "{} requires a finite, non-negative speed_bonus_per_level",
                path.display()
            ));
        }
        Ok(Self { config })
    }

    pub(crate) fn apply_speed(&self, player: &Player, experience_level: i32) {
        let Some(living) = player.as_entity().as_living() else {
            return;
        };
        // The built-in attribute has a maximum value of 1024.
        let bonus =
            (f64::from(experience_level.max(0)) * self.config.speed_bonus_per_level).min(1023.0);
        if bonus == 0.0 {
            living.remove_attribute_modifier(Attribute::BlockBreakSpeed, SPEED_MODIFIER_ID);
        } else {
            living.add_attribute_modifier(
                Attribute::BlockBreakSpeed,
                &AttributeModifier {
                    id: SPEED_MODIFIER_ID.into(),
                    amount: bonus,
                    operation: ModifierOperation::MultiplyTotal,
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_the_default_mining_speed_bonus() {
        let config: MiningConfig = toml::from_str(DEFAULT_CONFIG).unwrap();
        assert_eq!(config.speed_bonus_per_level, 0.1);
    }
}
