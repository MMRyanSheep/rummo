use std::collections::HashMap;
use std::mem;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use pumpkin_plugin_api::boss_bar::{BossBar, BossBarColor, BossBarDivision};
use pumpkin_plugin_api::text::TextComponent;
use pumpkin_plugin_api::{Player, Result};
use serde::Deserialize;

use crate::config;
use crate::skills::skill_manager::{SkillInfo, SkillType, player_uuid};

const DEFAULT_CONFIG: &str = include_str!("../../config/bossbar.toml");

#[derive(Deserialize)]
struct BossBarConfig {
    color: Option<String>,
}

#[derive(Default)]
struct BossBarState {
    bars: HashMap<u128, Arc<PlayerBossBar>>,
    dirty: HashMap<u128, (SkillType, SkillInfo)>,
    last_experience_update: HashMap<u128, Instant>,
}

pub(crate) struct BossBarManager {
    color: BossBarColor,
    state: Mutex<BossBarState>,
}

impl BossBarManager {
    pub(crate) fn load(path: &Path) -> Result<Self> {
        let config: BossBarConfig = config::load(path, DEFAULT_CONFIG)?;
        let color = match config.color.as_deref().unwrap_or("green") {
            "pink" => BossBarColor::Pink,
            "blue" => BossBarColor::Blue,
            "red" => BossBarColor::Red,
            "green" => BossBarColor::Green,
            "yellow" => BossBarColor::Yellow,
            "purple" => BossBarColor::Purple,
            "white" => BossBarColor::White,
            color => {
                return Err(format!(
                    "Invalid boss bar color {color:?} in {}",
                    path.display()
                ));
            }
        };
        Ok(Self {
            color,
            state: Mutex::default(),
        })
    }

    pub(crate) fn add_player(&self, player: &Player) {
        let uuid = player_uuid(player);
        let bar = Arc::new(PlayerBossBar {
            bar: BossBar::new(
                TextComponent::text(""),
                self.color,
                BossBarDivision::NoDivision,
            ),
            visible: AtomicBool::new(false),
        });
        self.state
            .lock()
            .expect("boss bar state lock")
            .bars
            .entry(uuid)
            .or_insert_with(|| Arc::clone(&bar));
    }

    pub(crate) fn remove_player(&self, player: &Player) {
        let uuid = player_uuid(player);
        let removed = {
            let mut state = self.state.lock().expect("boss bar state lock");
            state.dirty.remove(&uuid);
            state.last_experience_update.remove(&uuid);
            state.bars.remove(&uuid)
        };
        drop(removed);
    }

    pub(crate) fn experience_updated(&self, skill: SkillType, info: SkillInfo) {
        let now = Instant::now();
        let mut state = self.state.lock().expect("boss bar state lock");
        if state.bars.contains_key(&info.player_uuid) {
            state.dirty.insert(info.player_uuid, (skill, info));
            state.last_experience_update.insert(info.player_uuid, now);
        }
    }

    pub(super) fn dirty(&self) -> Vec<(Arc<PlayerBossBar>, SkillType, SkillInfo)> {
        let state = self.state.lock().expect("boss bar state lock");
        state
            .dirty
            .iter()
            .map(|(uuid, &(skill, info))| (Arc::clone(&state.bars[uuid]), skill, info))
            .collect()
    }

    pub(super) fn clear_rendered(
        &self,
        bar: &Arc<PlayerBossBar>,
        skill: SkillType,
        info: SkillInfo,
    ) {
        let mut state = self.state.lock().expect("boss bar state lock");
        let uuid = info.player_uuid;
        // Rendering can yield to an experience update or a player reconnect.
        if state.dirty.get(&uuid) == Some(&(skill, info))
            && state
                .bars
                .get(&uuid)
                .is_some_and(|current| Arc::ptr_eq(current, bar))
        {
            state.dirty.remove(&uuid);
        }
    }

    pub(super) fn hide_expired(&self, timeout: Duration) {
        let now = Instant::now();
        let expired = {
            let mut state = self.state.lock().expect("boss bar state lock");
            let BossBarState {
                bars,
                dirty,
                last_experience_update,
            } = &mut *state;
            last_experience_update
                .extract_if(|_, updated_at| now.saturating_duration_since(*updated_at) > timeout)
                .map(|(uuid, _)| {
                    dirty.remove(&uuid);
                    Arc::clone(&bars[&uuid])
                })
                .collect::<Vec<_>>()
        };
        for bar in expired {
            bar.hide();
        }
    }

    pub(crate) fn shutdown(&self) {
        let state = mem::take(&mut *self.state.lock().expect("boss bar state lock"));
        drop(state);
    }
}

pub(super) struct PlayerBossBar {
    pub(super) bar: BossBar,
    visible: AtomicBool,
}

impl PlayerBossBar {
    pub(super) fn is_visible(&self) -> bool {
        self.visible.load(Ordering::Relaxed)
    }

    pub(super) fn show(&self, player: Player) {
        self.bar.add_player(player);
        self.visible.store(true, Ordering::Relaxed);
    }

    fn hide(&self) {
        if self.visible.swap(false, Ordering::Relaxed) {
            for player in self.bar.get_players() {
                self.bar.remove_player(player);
            }
        }
    }
}

impl Drop for PlayerBossBar {
    fn drop(&mut self) {
        if *self.visible.get_mut() {
            self.bar.remove_all();
        }
    }
}
