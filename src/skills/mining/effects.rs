use std::collections::{HashMap, hash_map::RandomState};
use std::fs;
use std::hash::BuildHasher;
use std::io::ErrorKind;
use std::path::Path;
use std::sync::Mutex;

use pumpkin_plugin_api::enchantment::Enchantment;
use pumpkin_plugin_api::{
    Attribute, AttributeModifier, ItemStack, ModifierOperation, Player, Result,
};
use serde::Deserialize;

use crate::skills::skill_manager::player_uuid;

const DEFAULT_CONFIG: &str = include_str!("../../../config/mining.toml");
const SPEED_MODIFIER_ID: &str = "rummo:mining_speed";
const MAX_FORTUNE_LEVEL: u32 = 255;

#[derive(Deserialize)]
#[serde(default)]
struct MiningConfig {
    speed_bonus_per_level: f64,
    fortune_bonus_per_level: f64,
}

impl Default for MiningConfig {
    fn default() -> Self {
        Self {
            speed_bonus_per_level: 0.1,
            fortune_bonus_per_level: 0.01,
        }
    }
}

pub(crate) struct MiningEffects {
    config: MiningConfig,
    fortune: Mutex<HashMap<u128, f64>>,
}

impl MiningEffects {
    pub(crate) fn load(path: &Path) -> Result<Self> {
        let contents = match fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                fs::write(path, DEFAULT_CONFIG)
                    .map_err(|error| format!("Cannot create {}: {error}", path.display()))?;
                DEFAULT_CONFIG.to_owned()
            }
            Err(error) => return Err(format!("Cannot read {}: {error}", path.display())),
        };
        let config: MiningConfig = toml::from_str(&contents)
            .map_err(|error| format!("Cannot parse {}: {error}", path.display()))?;
        for (key, value) in [
            ("speed_bonus_per_level", config.speed_bonus_per_level),
            ("fortune_bonus_per_level", config.fortune_bonus_per_level),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(format!(
                    "{} requires a finite, non-negative {key}",
                    path.display()
                ));
            }
        }
        Ok(Self {
            config,
            fortune: Mutex::new(HashMap::new()),
        })
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

    pub(crate) fn update_fortune(&self, player_uuid: u128, mining_level: u32) {
        let bonus = (f64::from(mining_level) * self.config.fortune_bonus_per_level)
            .min(f64::from(MAX_FORTUNE_LEVEL));
        self.fortune
            .lock()
            .expect("mining fortune lock")
            .insert(player_uuid, bonus);
    }

    pub(crate) fn remove_fortune(&self, player: &Player) {
        let player_uuid = player_uuid(player);
        self.fortune
            .lock()
            .expect("mining fortune lock")
            .remove(&player_uuid);
    }

    pub(super) fn bonus_fortune_level(&self, player_uuid: u128, tool: &ItemStack) -> Option<u32> {
        let bonus = self
            .fortune
            .lock()
            .expect("mining fortune lock")
            .get(&player_uuid)
            .copied()
            .unwrap_or_default();
        if bonus == 0.0 {
            return None;
        }

        // Use the top 53 bits so the draw is representable as a value in [0, 1).
        let bits = RandomState::new().hash_one(player_uuid) >> 11;
        let draw = bits as f64 / (1_u64 << 53) as f64;
        let bonus_level = rounded_fortune(bonus, draw);
        if bonus_level == 0 {
            return None;
        }

        let mut tool_level = 0;
        for enchantment in tool.get_enchantments() {
            match enchantment.enchantment {
                Enchantment::SilkTouch if enchantment.level > 0 => return None,
                Enchantment::Fortune => tool_level = enchantment.level,
                _ => {}
            }
        }
        let effective_level = tool_level
            .saturating_add(bonus_level)
            .min(MAX_FORTUNE_LEVEL);
        (effective_level > tool_level).then_some(effective_level)
    }
}

fn rounded_fortune(bonus: f64, draw: f64) -> u32 {
    bonus as u32 + u32::from(draw < bonus.fract())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rolls_fractional_fortune_at_the_configured_probability() {
        let config: MiningConfig = toml::from_str(DEFAULT_CONFIG).unwrap();
        assert_eq!(config.speed_bonus_per_level, 0.1);
        let bonus = 150.0 * config.fortune_bonus_per_level;
        assert_eq!(rounded_fortune(bonus, 0.49), 2);
        assert_eq!(rounded_fortune(bonus, 0.5), 1);
        assert_eq!(rounded_fortune(1.0, 0.0), 1);
    }
}
