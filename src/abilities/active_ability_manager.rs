use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use pumpkin_plugin_api::Player;

use super::ability_manager::AbilityId;
use super::ability_registry::AbilityRegistry;
use super::cooldown_manager::COOLDOWNS;
use super::mining::super_breaker::activate_super_breaker;
use crate::skills::skill_manager::player_uuid;

type Uuid = u128;

static ACTIVE_ABILITY_MANAGER: LazyLock<Mutex<ActiveAbilityManager>> =
    LazyLock::new(|| Mutex::new(ActiveAbilityManager::default()));

#[derive(Default)]
pub(crate) struct ActiveAbilityManager {
    expiration_queue: BinaryHeap<Reverse<ExpirationEntry>>,
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "Expiration entries are not processed yet")
)]
pub(crate) struct ExpirationEntry {
    pub expires_at: Instant,
    pub uuid: Uuid,
    pub ability_id: AbilityId,
}

impl Ord for ExpirationEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.expires_at.cmp(&other.expires_at)
    }
}

impl PartialOrd for ExpirationEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for ExpirationEntry {
    fn eq(&self, other: &Self) -> bool {
        self.expires_at == other.expires_at
    }
}

impl Eq for ExpirationEntry {}

impl ActiveAbilityManager {
    fn record_activation(
        &mut self,
        uuid: Uuid,
        ability_id: AbilityId,
        duration: Duration,
    ) -> Result<(), &'static str> {
        if duration.is_zero() {
            return Ok(());
        }

        let expires_at = Instant::now()
            .checked_add(duration)
            .ok_or("Ability duration is too large")?;
        self.expiration_queue.push(Reverse(ExpirationEntry {
            expires_at,
            uuid,
            ability_id,
        }));
        Ok(())
    }
}

pub(crate) fn activate(registry: &AbilityRegistry, ability_id: AbilityId, player: &Player) {
    let Some(definition) = registry.get(ability_id) else {
        return;
    };
    let player_id = player_uuid(player);
    let is_ready = COOLDOWNS
        .lock()
        .expect("cooldown manager lock")
        .is_ready(player_id, ability_id);
    if !is_ready {
        return;
    }

    let result = ACTIVE_ABILITY_MANAGER
        .lock()
        .expect("active ability manager lock")
        .record_activation(player_id, ability_id, definition.duration);
    if let Err(error) = result {
        tracing::error!(?ability_id, %error, "Failed to activate ability");
        return;
    }

    match ability_id {
        AbilityId::SuperBreaker => activate_super_breaker(player),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_only_activated_duration_abilities() {
        let mut manager = ActiveAbilityManager::default();
        let ability_id = AbilityId::SuperBreaker;
        let duration = Duration::from_secs(4);
        let started = Instant::now();

        manager.record_activation(1, ability_id, duration).unwrap();
        manager
            .record_activation(2, ability_id, Duration::ZERO)
            .unwrap();

        let Reverse(expiration) = manager.expiration_queue.pop().unwrap();
        assert!(expiration.expires_at >= started + duration);
        assert!(expiration.expires_at <= Instant::now() + duration);
        assert_eq!(expiration.uuid, 1);
        assert_eq!(expiration.ability_id, ability_id);
        assert!(manager.expiration_queue.is_empty());
    }

    #[test]
    fn queues_earliest_expiration_first() {
        let now = Instant::now();
        let ability_id = AbilityId::SuperBreaker;
        let queue = BinaryHeap::from([
            Reverse(ExpirationEntry {
                expires_at: now + Duration::from_secs(1),
                uuid: 1,
                ability_id,
            }),
            Reverse(ExpirationEntry {
                expires_at: now,
                uuid: 2,
                ability_id,
            }),
        ]);

        assert_eq!(queue.peek().unwrap().0.uuid, 2);
    }
}
