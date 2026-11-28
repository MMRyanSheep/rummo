use std::sync::Mutex;
use std::time::{Duration, Instant};

use pumpkin_plugin_api::Server;
use pumpkin_plugin_api::scheduler::{cancel_task, schedule_delayed_task};

use super::ability_manager::AbilityId;
use super::active_ability_manager::ACTIVE_ABILITY_MANAGER;
use super::mining::super_breaker::deactivate_super_breaker;
use super::unarmed::berserk::deactivate_berserk;
use crate::skills::skill_manager::to_pumpkin_uuid;

const TICK_DURATION: Duration = Duration::from_millis(50);

static LIFECYCLE: Mutex<AbilityLifecycleService> = Mutex::new(AbilityLifecycleService {
    generation: 0,
    wakeup: None,
});

pub(super) struct AbilityLifecycleService {
    generation: u64,
    wakeup: Option<Wakeup>,
}

struct Wakeup {
    expires_at: Instant,
    task_id: Option<u32>,
}

impl AbilityLifecycleService {
    pub(super) fn schedule_next() {
        let expires_at = ACTIVE_ABILITY_MANAGER
            .lock()
            .expect("active ability manager lock")
            .next_expiration();
        let Some(expires_at) = expires_at else {
            return;
        };

        let reservation = LIFECYCLE
            .lock()
            .expect("ability lifecycle lock")
            .reserve(expires_at);
        let Some((generation, previous_task)) = reservation else {
            return;
        };

        if let Some(task_id) = previous_task {
            cancel_task(task_id);
        }
        let delay = expires_at.saturating_duration_since(Instant::now());
        let task_id = schedule_delayed_task(delay_ticks(delay), move |server| {
            Self::wake(&server, generation);
        });

        // Scheduling can yield to a callback that replaces or consumes this wakeup.
        let remembered = LIFECYCLE
            .lock()
            .expect("ability lifecycle lock")
            .remember_task(generation, task_id);
        if !remembered {
            cancel_task(task_id);
        }
    }

    fn reserve(&mut self, expires_at: Instant) -> Option<(u64, Option<u32>)> {
        if self
            .wakeup
            .as_ref()
            .is_some_and(|wake| wake.expires_at <= expires_at)
        {
            return None;
        }

        self.generation = self.generation.wrapping_add(1);
        let previous = self.wakeup.replace(Wakeup {
            expires_at,
            task_id: None,
        });
        Some((self.generation, previous.and_then(|wake| wake.task_id)))
    }

    fn remember_task(&mut self, generation: u64, task_id: u32) -> bool {
        if self.generation == generation
            && let Some(wakeup) = &mut self.wakeup
        {
            wakeup.task_id = Some(task_id);
            return true;
        }
        false
    }

    fn claim(&mut self, generation: u64) -> bool {
        self.generation == generation && self.wakeup.take().is_some()
    }

    fn wake(server: &Server, generation: u64) {
        let claimed = LIFECYCLE
            .lock()
            .expect("ability lifecycle lock")
            .claim(generation);
        if !claimed {
            return;
        }

        let now = Instant::now();
        let expired = ACTIVE_ABILITY_MANAGER
            .lock()
            .expect("active ability manager lock")
            .take_expired(now);
        for entry in expired {
            let Some(player) = server.get_player_by_uuid(to_pumpkin_uuid(entry.uuid)) else {
                continue;
            };
            match entry.ability_id {
                AbilityId::SuperBreaker => deactivate_super_breaker(&player),
                AbilityId::Berserk => deactivate_berserk(&player),
            }
        }

        Self::schedule_next();
    }
}

fn delay_ticks(delay: Duration) -> u64 {
    let ticks = delay.as_nanos().div_ceil(TICK_DURATION.as_nanos());
    u64::try_from(ticks).unwrap_or(u64::MAX).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_a_wakeup_only_for_an_earlier_expiration() {
        let mut lifecycle = AbilityLifecycleService {
            generation: 0,
            wakeup: None,
        };
        let earlier = Instant::now();
        let later = earlier + TICK_DURATION;

        let (first, previous) = lifecycle.reserve(later).unwrap();
        assert_eq!(previous, None);
        assert!(lifecycle.remember_task(first, 1));
        assert!(lifecycle.reserve(later).is_none());

        let (second, previous) = lifecycle.reserve(earlier).unwrap();
        assert_eq!(previous, Some(1));
        assert!(lifecycle.reserve(later).is_none());
        assert!(!lifecycle.remember_task(first, 2));
        assert!(!lifecycle.claim(first));
        assert!(lifecycle.claim(second));
        assert!(!lifecycle.remember_task(second, 3));
        assert!(lifecycle.reserve(later).is_some());
    }

    #[test]
    fn rounds_delay_up_to_the_next_tick() {
        assert_eq!(delay_ticks(Duration::ZERO), 1);
        assert_eq!(delay_ticks(TICK_DURATION), 1);
        assert_eq!(delay_ticks(TICK_DURATION + Duration::from_nanos(1)), 2);
    }
}
