use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use pumpkin_plugin_api::Player;

use super::ability_definition::AbilityDefinition;
use super::ability_manager::AbilityId;
use crate::skills::skill_manager::SkillManager;

type Uuid = u128;

pub(super) static COOLDOWNS: LazyLock<Mutex<CooldownManager>> =
    LazyLock::new(|| Mutex::new(CooldownManager::default()));

#[derive(Default)]
pub struct CooldownManager {
    cooldowns: HashMap<Uuid, HashMap<AbilityId, Instant>>,
}

impl CooldownManager {
    fn calculate_cooldown(player: &Player, definition: &AbilityDefinition) -> Duration {
        match SkillManager::get_skill(player, definition.parent_skill) {
            Ok(skill) => Self::cooldown_at_level(definition, skill.level),
            Err(error) => {
                tracing::error!(%error, "Failed to read skill level for ability cooldown");
                definition.base_cooldown
            }
        }
    }

    fn cooldown_at_level(definition: &AbilityDefinition, level: u32) -> Duration {
        let reduction = definition.cooldown_per_level.saturating_mul(level);
        definition
            .base_cooldown
            .saturating_sub(reduction)
            .max(definition.min_cooldown)
    }

    /// Starts a cooldown when ready or returns the remaining wait.
    pub(crate) fn start_cooldown(
        player: &Player,
        player_id: Uuid,
        ability_id: AbilityId,
        definition: &AbilityDefinition,
    ) -> Result<Option<Duration>, &'static str> {
        let duration = Self::calculate_cooldown(player, definition);
        let now = Instant::now();
        COOLDOWNS
            .lock()
            .expect("cooldown manager lock")
            .start_if_ready(player_id, ability_id, duration, now)
    }

    fn start_if_ready(
        &mut self,
        player_id: Uuid,
        ability_id: AbilityId,
        duration: Duration,
        now: Instant,
    ) -> Result<Option<Duration>, &'static str> {
        let remaining = self.remaining(player_id, ability_id, now);
        if !remaining.is_zero() {
            return Ok(Some(remaining));
        }

        let end = now
            .checked_add(duration)
            .ok_or("Cooldown duration is too large")?;
        self.cooldowns
            .entry(player_id)
            .or_default()
            .insert(ability_id, end);
        Ok(None)
    }

    fn remaining(&self, player_id: Uuid, ability_id: AbilityId, now: Instant) -> Duration {
        self.cooldowns
            .get(&player_id)
            .and_then(|abilities| abilities.get(&ability_id))
            .map_or(Duration::ZERO, |end| end.saturating_duration_since(now))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::ability_registry::AbilityRegistry;

    #[test]
    fn cooldown_is_clamped_to_the_configured_minimum() {
        let registry: AbilityRegistry =
            toml::from_str(include_str!("../../config/abilities.toml")).unwrap();
        let definition = registry.get(AbilityId::SuperBreaker).unwrap();

        for (level, seconds) in [(10, 100), (45, 30), (46, 30), (60, 30), (u32::MAX, 30)] {
            assert_eq!(
                CooldownManager::cooldown_at_level(definition, level),
                Duration::from_secs(seconds)
            );
        }
    }

    #[test]
    fn tracks_cooldowns_by_player() {
        let mut manager = CooldownManager::default();
        let ability_id = AbilityId::SuperBreaker;
        let now = Instant::now();
        let duration = Duration::from_secs(30);
        assert_eq!(manager.remaining(1, ability_id, now), Duration::ZERO);

        assert_eq!(
            manager.start_if_ready(1, ability_id, duration, now),
            Ok(None)
        );

        assert_eq!(manager.remaining(1, ability_id, now), duration);
        assert_eq!(manager.remaining(2, ability_id, now), Duration::ZERO);
    }

    #[test]
    fn blocks_reactivation_until_expiry() {
        let mut manager = CooldownManager::default();
        let ability_id = AbilityId::SuperBreaker;
        let now = Instant::now();
        let duration = Duration::from_secs(30);
        assert_eq!(
            manager.start_if_ready(1, ability_id, duration, now),
            Ok(None)
        );

        let retry_time = now + Duration::from_secs(1);
        assert_eq!(
            manager.start_if_ready(1, ability_id, Duration::ZERO, retry_time),
            Ok(Some(Duration::from_secs(29)))
        );
        let expiry = now + duration;
        assert_eq!(
            manager.start_if_ready(1, ability_id, Duration::ZERO, expiry),
            Ok(None)
        );
        assert_eq!(manager.remaining(1, ability_id, expiry), Duration::ZERO);
    }
}
