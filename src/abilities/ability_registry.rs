use std::collections::HashMap;
use std::path::Path;

use pumpkin_plugin_api::Result;
use serde::Deserialize;

use super::ability_definition::AbilityDefinition;
use super::ability_manager::AbilityId;
use crate::config;

const DEFAULT_CONFIG: &str = include_str!("../../config/abilities.toml");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AbilityRegistry {
    abilities: HashMap<AbilityId, AbilityDefinition>,
}

impl AbilityRegistry {
    pub(crate) fn load(path: &Path) -> Result<Self> {
        let registry: Self = config::load(path, DEFAULT_CONFIG)?;
        registry
            .validate()
            .map_err(|error| format!("Cannot load {}: {error}", path.display()))?;
        Ok(registry)
    }

    pub(crate) fn get(&self, ability_id: AbilityId) -> Option<&AbilityDefinition> {
        self.abilities.get(&ability_id)
    }

    fn validate(&self) -> Result<()> {
        for (id, definition) in &self.abilities {
            if definition.min_cooldown > definition.base_cooldown {
                return Err(format!(
                    "Invalid settings for {id:?}: min_cooldown must not exceed base_cooldown"
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::skills::skill_manager::SkillType;

    #[test]
    fn registers_super_breaker_from_the_default_config() {
        let registry: AbilityRegistry = toml::from_str(DEFAULT_CONFIG).unwrap();
        registry.validate().unwrap();
        let definition = registry.get(AbilityId::SuperBreaker).unwrap();

        assert_eq!(definition.parent_skill, SkillType::Mining);
        assert_eq!(definition.unlock_level, 10);
        assert_eq!(definition.base_cooldown, Duration::from_secs(120));
        assert_eq!(definition.cooldown_per_level, Duration::from_secs(2));
        assert_eq!(definition.min_cooldown, Duration::from_secs(30));
    }

    #[test]
    fn loads_fractional_cooldown_seconds_as_a_duration() {
        let contents =
            DEFAULT_CONFIG.replace("cooldown_per_level = 2.0", "cooldown_per_level = 2.5");
        let registry: AbilityRegistry = toml::from_str(&contents).unwrap();
        let definition = registry.get(AbilityId::SuperBreaker).unwrap();

        assert_eq!(definition.cooldown_per_level, Duration::from_millis(2500));
    }

    #[test]
    fn rejects_invalid_ability_settings() {
        for (original, invalid, field) in [
            ("\"mining\"", "\"unknown\"", "parent_skill"),
            ("120.0", "-120.0", "base_cooldown"),
            ("2.0", "inf", "cooldown_per_level"),
            ("30.0", "121.0", "min_cooldown"),
        ] {
            let contents = DEFAULT_CONFIG.replace(original, invalid);
            let error = toml::from_str::<AbilityRegistry>(&contents)
                .map_err(|error| error.to_string())
                .and_then(|registry| registry.validate())
                .unwrap_err();
            assert!(error.contains(field), "{error}");
        }
    }
}
