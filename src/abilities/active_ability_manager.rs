use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;
use std::collections::binary_heap::PeekMut;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use pumpkin_plugin_api::Player;
use pumpkin_plugin_api::text::TextComponent;

use super::ability_lifecycle_service::AbilityLifecycleService;
use super::ability_manager::AbilityId;
use super::ability_registry::AbilityRegistry;
use super::mining::super_breaker::activate_super_breaker;
use crate::i18n;
use crate::skills::skill_manager::player_uuid;

type Uuid = u128;

pub(super) static ACTIVE_ABILITY_MANAGER: LazyLock<Mutex<ActiveAbilityManager>> =
    LazyLock::new(|| Mutex::new(ActiveAbilityManager::default()));

#[derive(Default)]
pub(crate) struct ActiveAbilityManager {
    expiration_queue: BinaryHeap<Reverse<ExpirationEntry>>,
}

pub(crate) struct ExpirationEntry {
    pub expires_at: Instant,
    pub uuid: Uuid,
    pub ability_id: AbilityId,
}

impl ExpirationEntry {
    fn for_activation(
        uuid: Uuid,
        ability_id: AbilityId,
        duration: Duration,
    ) -> Result<Option<Self>, &'static str> {
        if duration.is_zero() {
            return Ok(None);
        }

        let expires_at = Instant::now()
            .checked_add(duration)
            .ok_or("Ability duration is too large")?;
        Ok(Some(Self {
            expires_at,
            uuid,
            ability_id,
        }))
    }
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
    pub(super) fn next_expiration(&self) -> Option<Instant> {
        self.expiration_queue.peek().map(|entry| entry.0.expires_at)
    }

    pub(super) fn take_expired(&mut self, now: Instant) -> Vec<ExpirationEntry> {
        let mut expired = Vec::new();
        while let Some(entry) = self
            .expiration_queue
            .peek_mut()
            .filter(|entry| entry.0.expires_at <= now)
        {
            expired.push(PeekMut::pop(entry).0);
        }
        expired
    }
}

pub(crate) fn activate(registry: &AbilityRegistry, ability_id: AbilityId, player: &Player) {
    let Some(definition) = registry.get(ability_id) else {
        return;
    };
    let player_id = player_uuid(player);
    let expiration =
        match ExpirationEntry::for_activation(player_id, ability_id, definition.duration) {
            Ok(expiration) => expiration,
            Err(error) => {
                tracing::error!(?ability_id, %error, "Failed to activate ability");
                return;
            }
        };

    let activation = match ability_id {
        AbilityId::SuperBreaker => activate_super_breaker(player, player_id, definition),
    };
    match activation {
        Ok(true) => {}
        Ok(false) => return,
        Err(error) => {
            tracing::error!(?ability_id, %error, "Failed to activate ability");
            return;
        }
    }

    if let Some(expiration) = expiration {
        ACTIVE_ABILITY_MANAGER
            .lock()
            .expect("active ability manager lock")
            .expiration_queue
            .push(Reverse(expiration));
        AbilityLifecycleService::schedule_next();
    }
}

pub(super) fn show_cooldown(player: &Player, ability_id: AbilityId, remaining: Duration) {
    let seconds = remaining
        .as_nanos()
        .div_ceil(Duration::from_secs(1).as_nanos());
    let messages = i18n::for_player(player);
    player.show_subtitle(TextComponent::text(&messages.format(
        "ability.cooldown",
        &[
            ("seconds", &seconds),
            ("ability", &messages.text(ability_id.name_key())),
        ],
    )));
    player.show_title(TextComponent::text(""));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_expiration_only_for_duration_abilities() {
        let ability_id = AbilityId::SuperBreaker;
        let duration = Duration::from_secs(4);
        let started = Instant::now();

        let expiration = ExpirationEntry::for_activation(1, ability_id, duration)
            .unwrap()
            .unwrap();
        assert!(
            ExpirationEntry::for_activation(2, ability_id, Duration::ZERO)
                .unwrap()
                .is_none()
        );

        assert!(expiration.expires_at >= started + duration);
        assert!(expiration.expires_at <= Instant::now() + duration);
        assert_eq!(expiration.uuid, 1);
        assert_eq!(expiration.ability_id, ability_id);
    }

    #[test]
    fn drains_due_expirations_in_order() {
        let now = Instant::now();
        let ability_id = AbilityId::SuperBreaker;
        let later = now + Duration::from_secs(1);
        let expiration_queue = BinaryHeap::from([
            Reverse(ExpirationEntry {
                expires_at: later,
                uuid: 1,
                ability_id,
            }),
            Reverse(ExpirationEntry {
                expires_at: now,
                uuid: 2,
                ability_id,
            }),
            Reverse(ExpirationEntry {
                expires_at: now - Duration::from_secs(1),
                uuid: 3,
                ability_id,
            }),
        ]);

        let mut manager = ActiveAbilityManager { expiration_queue };
        let expired = manager.take_expired(now);
        assert_eq!(
            expired.iter().map(|entry| entry.uuid).collect::<Vec<_>>(),
            [3, 2]
        );
        assert_eq!(manager.next_expiration(), Some(later));
        assert_eq!(manager.take_expired(later)[0].uuid, 1);
        assert_eq!(manager.next_expiration(), None);
    }
}
