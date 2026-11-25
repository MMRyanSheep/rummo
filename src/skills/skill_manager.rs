use std::path::Path;
use std::sync::{Arc, Mutex};

use pumpkin_plugin_api::uuid::Uuid;
use pumpkin_plugin_api::{Context, Player, Result};

use super::experience::ExperienceConfig;
use crate::storage::{Progress, SkillKey, SkillStorage};
use crate::ui::bossbar_manager::BossBarManager;

static MANAGER: Mutex<Option<SkillManager>> = Mutex::new(None);
type LevelChangeHandler = Arc<dyn Fn(&Player, SkillType) + Send + Sync>;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SkillType {
    Mining,
    Axes,
}

impl SkillType {
    pub const fn id(self) -> &'static str {
        match self {
            Self::Mining => "mining",
            Self::Axes => "axes",
        }
    }

    pub(crate) fn from_id(id: &str) -> Option<Self> {
        match id {
            "mining" => Some(Self::Mining),
            "axes" => Some(Self::Axes),
            _ => None,
        }
    }

    pub(crate) const fn display_name(self) -> &'static str {
        match self {
            Self::Mining => "挖矿",
            Self::Axes => "斧技",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SkillInfo {
    pub player_uuid: u128,
    pub skill_id: &'static str,
    pub level: u32,
    /// Progress within the current level.
    pub experience: u32,
    /// Total experience required to advance from the current level.
    pub required_experience: Option<u32>,
}

pub struct SkillManager {
    boss_bars: Arc<BossBarManager>,
    config: ExperienceConfig,
    storage: Arc<SkillStorage>,
    level_change_handler: Option<LevelChangeHandler>,
}

impl SkillManager {
    /// Adds experience, carrying any surplus across levels.
    pub fn add_exp(player: &Player, skill_type: SkillType, exp: u32) -> Result<SkillInfo> {
        Self::update_skill(player, skill_type, |manager, key| {
            manager.update_experience(key, |current| u64::from(current) + u64::from(exp))
        })
    }

    /// Replaces the current level's progress. Surplus experience advances levels.
    pub fn set_exp(player: &Player, skill_type: SkillType, exp: u32) -> Result<SkillInfo> {
        Self::update_skill(player, skill_type, |manager, key| {
            manager.update_experience(key, |_| u64::from(exp))
        })
    }

    /// Sets the level and keeps as much current progress as the target level allows.
    pub(crate) fn set_level(
        player: &Player,
        skill_type: SkillType,
        level: u32,
    ) -> Result<SkillInfo> {
        if level == 0 {
            return Err("Skill level must be at least 1".into());
        }
        Self::update_skill(player, skill_type, |manager, key| {
            manager.update_level(key, |_| level)
        })
    }

    /// Adds levels up to the maximum while preserving current progress.
    pub(crate) fn add_level(
        player: &Player,
        skill_type: SkillType,
        levels: u32,
    ) -> Result<SkillInfo> {
        Self::update_skill(player, skill_type, |manager, key| {
            manager.update_level(key, |current| current.saturating_add(levels))
        })
    }

    pub fn get_skill(player: &Player, skill_type: SkillType) -> Result<SkillInfo> {
        let key = skill_key(player, skill_type);
        with_manager(|manager| manager.info(key, manager.storage.progress(key)))
    }

    pub(crate) fn initialize(context: &Context, boss_bars: Arc<BossBarManager>) -> Result<()> {
        let data_folder = context.get_data_folder();
        let directory = Path::new(&data_folder);
        let manager = Self::open(directory, boss_bars)?;
        let storage = Arc::clone(&manager.storage);
        *MANAGER.lock().expect("skill manager lock") = Some(manager);
        storage.schedule_autosave(context);
        Ok(())
    }

    pub(crate) fn set_level_change_handler(handler: LevelChangeHandler) -> Result<()> {
        let mut manager = MANAGER.lock().expect("skill manager lock");
        let manager = manager
            .as_mut()
            .ok_or_else(|| "Skill manager is not loaded".to_owned())?;
        manager.level_change_handler = Some(handler);
        Ok(())
    }

    pub(crate) fn shutdown() -> Result<()> {
        let manager = MANAGER.lock().expect("skill manager lock").take();
        if let Some(manager) = manager {
            manager.boss_bars.shutdown();
            manager.storage.shutdown()?;
        }
        Ok(())
    }
    fn open(directory: &Path, boss_bars: Arc<BossBarManager>) -> Result<Self> {
        let config = ExperienceConfig::load(&directory.join("experience.toml"))?;
        let storage = Arc::new(SkillStorage::open(directory)?);
        Ok(Self {
            boss_bars,
            config,
            storage,
            level_change_handler: None,
        })
    }

    fn update_skill(
        player: &Player,
        skill_type: SkillType,
        update: impl FnOnce(&Self, SkillKey) -> (SkillInfo, bool),
    ) -> Result<SkillInfo> {
        let key = skill_key(player, skill_type);
        let (info, handler) = with_manager(|manager| {
            let (info, level_changed) = update(manager, key);
            let handler = if level_changed {
                manager.level_change_handler.clone()
            } else {
                None
            };
            (info, handler)
        })?;
        if let Some(handler) = handler {
            handler(player, skill_type);
        }
        Ok(info)
    }

    fn update_experience(
        &self,
        key: SkillKey,
        experience: impl FnOnce(u32) -> u64,
    ) -> (SkillInfo, bool) {
        self.update_progress(key, |current| {
            self.config
                .advance(current.level, experience(current.experience))
        })
    }

    fn update_level(&self, key: SkillKey, level: impl FnOnce(u32) -> u32) -> (SkillInfo, bool) {
        self.update_progress(key, |current| {
            self.config
                .progress_at_level(level(current.level), current.experience)
        })
    }

    fn update_progress(
        &self,
        key: SkillKey,
        update: impl FnOnce(Progress) -> Progress,
    ) -> (SkillInfo, bool) {
        let previous = self.storage.progress(key);
        let progress = self.storage.update(key, update);
        let info = self.info(key, progress);
        if progress != previous {
            self.boss_bars.experience_updated(key.1, info);
        }
        (info, progress.level != previous.level)
    }

    fn info(&self, (player_uuid, skill_type): SkillKey, progress: Progress) -> SkillInfo {
        SkillInfo {
            player_uuid,
            skill_id: skill_type.id(),
            level: progress.level,
            experience: progress.experience,
            required_experience: (progress.level < u32::MAX)
                .then(|| self.config.required_experience(progress.level)),
        }
    }
}

fn skill_key(player: &Player, skill_type: SkillType) -> SkillKey {
    (player_uuid(player), skill_type)
}

pub(crate) fn player_uuid(player: &Player) -> u128 {
    let uuid = player.get_id();
    (u128::from(uuid.high) << 64) | u128::from(uuid.low)
}

pub(crate) fn to_pumpkin_uuid(uuid: u128) -> Uuid {
    Uuid {
        high: (uuid >> 64) as u64,
        low: uuid as u64,
    }
}

fn with_manager<T>(use_manager: impl FnOnce(&SkillManager) -> T) -> Result<T> {
    MANAGER
        .lock()
        .expect("skill manager lock")
        .as_ref()
        .map(use_manager)
        .ok_or_else(|| "Skill manager is not loaded".into())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn persists_level_and_experience_on_flush_and_shutdown() {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let directory = std::env::temp_dir().join(format!(
            "rummo-skills-{}-{}",
            std::process::id(),
            timestamp.as_nanos()
        ));
        fs::create_dir(&directory).unwrap();
        let boss_bars = Arc::new(BossBarManager::load(&directory.join("bossbar.toml")).unwrap());
        let manager = SkillManager::open(&directory, boss_bars).unwrap();
        let key = (1, SkillType::Mining);

        let (skill, level_changed) = manager.update_experience(key, |_| 215);
        assert_eq!((skill.level, skill.experience), (3, 5));
        assert_eq!(skill.required_experience, Some(121));
        assert!(level_changed);
        let saved = SkillStorage::open(&directory).unwrap().progress(key);
        assert_eq!(saved, Progress::default());

        manager.storage.flush_pending().unwrap();
        let saved = SkillStorage::open(&directory).unwrap().progress(key);
        assert_eq!(
            saved,
            Progress {
                level: 3,
                experience: 5
            }
        );

        let axes_key = (1, SkillType::Axes);
        let (_, level_changed) = manager.update_experience(axes_key, |_| 4);
        assert!(!level_changed);
        manager.update_experience(axes_key, |_| 105);
        manager.update_experience(key, |_| 121 + 110);
        let (skill, level_changed) = manager.update_level(key, |_| 1);
        assert_eq!((skill.level, skill.experience), (1, 99));
        assert!(level_changed);
        *MANAGER.lock().unwrap() = Some(manager);
        SkillManager::shutdown().unwrap();
        let storage = SkillStorage::open(&directory).unwrap();
        assert_eq!(
            storage.progress(key),
            Progress {
                level: 1,
                experience: 99
            }
        );
        assert_eq!(
            storage.progress(axes_key),
            Progress {
                level: 2,
                experience: 5
            }
        );

        fs::remove_file(directory.join("skills.redb")).unwrap();
        fs::remove_file(directory.join("experience.toml")).unwrap();
        fs::remove_file(directory.join("bossbar.toml")).unwrap();
        fs::remove_dir(directory).unwrap();
    }
}
