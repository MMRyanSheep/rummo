use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use super::ability_manager::AbilityId;

type Uuid = u128;

pub(super) static COOLDOWNS: LazyLock<Mutex<CooldownManager>> =
    LazyLock::new(|| Mutex::new(CooldownManager::default()));

#[derive(Default)]
pub struct CooldownManager {
    cooldowns: HashMap<Uuid, HashMap<AbilityId, Instant>>,
}

impl CooldownManager {
    pub(crate) fn is_ready(&self, player_id: Uuid, ability_id: AbilityId) -> bool {
        self.ends_at(player_id, ability_id)
            .is_none_or(|end| end <= Instant::now())
    }

    pub(crate) fn remaining(&self, player_id: Uuid, ability_id: AbilityId) -> Duration {
        self.ends_at(player_id, ability_id)
            .map_or(Duration::ZERO, |end| {
                end.saturating_duration_since(Instant::now())
            })
    }

    pub(crate) fn start_cooldown(
        &mut self,
        player_id: Uuid,
        ability_id: AbilityId,
        duration: Duration,
    ) -> Result<(), &'static str> {
        let end = Instant::now()
            .checked_add(duration)
            .ok_or("Cooldown duration is too large")?;
        self.cooldowns
            .entry(player_id)
            .or_default()
            .insert(ability_id, end);
        Ok(())
    }

    fn ends_at(&self, player_id: Uuid, ability_id: AbilityId) -> Option<Instant> {
        self.cooldowns.get(&player_id)?.get(&ability_id).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_cooldowns_by_player() {
        let mut manager = CooldownManager::default();
        let ability_id = AbilityId::SuperBreaker;
        let duration = Duration::from_secs(30);
        assert!(manager.is_ready(1, ability_id));
        assert_eq!(manager.remaining(1, ability_id), Duration::ZERO);

        manager.start_cooldown(1, ability_id, duration).unwrap();

        assert!(!manager.is_ready(1, ability_id));
        let remaining = manager.remaining(1, ability_id);
        assert!(remaining > Duration::ZERO && remaining <= duration);
        assert!(manager.is_ready(2, ability_id));
    }

    #[test]
    fn replaces_existing_cooldown_and_treats_it_as_ready_at_expiry() {
        let mut manager = CooldownManager::default();
        let ability_id = AbilityId::SuperBreaker;
        manager
            .start_cooldown(1, ability_id, Duration::from_secs(30))
            .unwrap();

        manager
            .start_cooldown(1, ability_id, Duration::ZERO)
            .unwrap();

        assert!(manager.is_ready(1, ability_id));
        assert_eq!(manager.remaining(1, ability_id), Duration::ZERO);
    }
}
