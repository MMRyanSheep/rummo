use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::sync::{Arc, Mutex};

use pumpkin_plugin_api::{Context, Player, Result};
use serde::Deserialize;

use super::mining::MiningEffects;
use crate::storage::{Progress, SkillKey, SkillStorage};
use crate::ui::bossbar_manager::BossBarManager;

static MANAGER: Mutex<Option<SkillManager>> = Mutex::new(None);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SkillType {
    Mining,
}

impl SkillType {
    pub const fn id(self) -> &'static str {
        match self {
            Self::Mining => "mining",
        }
    }

    pub(crate) fn from_id(id: &str) -> Option<Self> {
        match id {
            "mining" => Some(Self::Mining),
            _ => None,
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

/// Mutually exclusive drop chances as percentages.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DropProbabilities {
    pub normal: u8,
    pub double: u8,
    pub triple: u8,
}

impl DropProbabilities {
    pub(crate) const TOTAL: u8 = 100;

    fn for_mining_level(level: u32) -> Self {
        let total = u32::from(Self::TOTAL);
        let triple = level.saturating_mul(5).min(total);
        let double = level.saturating_mul(15).min(total - triple);
        Self {
            normal: (total - triple - double) as u8,
            double: double as u8,
            triple: triple as u8,
        }
    }
}

pub struct SkillManager {
    boss_bars: Arc<BossBarManager>,
    config: ExperienceConfig,
    storage: Arc<SkillStorage>,
}

#[allow(non_snake_case)]
impl SkillManager {
    /// Adds experience, carrying any surplus across levels.
    pub fn addExp(player: &Player, skill_type: SkillType, exp: u32) -> Result<SkillInfo> {
        let key = skill_key(player, skill_type);
        with_manager(|manager| {
            manager.update_experience(key, |current| u64::from(current) + u64::from(exp))
        })
    }

    /// Replaces the current level's progress. Surplus experience advances levels.
    pub fn setExp(player: &Player, skill_type: SkillType, exp: u32) -> Result<SkillInfo> {
        let key = skill_key(player, skill_type);
        with_manager(|manager| manager.update_experience(key, |_| u64::from(exp)))
    }

    pub fn getSkill(player: &Player, skill_type: SkillType) -> Result<SkillInfo> {
        let key = skill_key(player, skill_type);
        with_manager(|manager| manager.info(key, manager.storage.progress(key)))
    }

    pub fn mining_drop_probabilities(player: &Player) -> Result<DropProbabilities> {
        let key = skill_key(player, SkillType::Mining);
        with_manager(|manager| {
            DropProbabilities::for_mining_level(manager.storage.progress(key).level)
        })
    }

    pub(crate) fn initialize(
        context: &Context,
    ) -> Result<(Arc<MiningEffects>, Arc<BossBarManager>)> {
        let data_folder = context.get_data_folder();
        let directory = Path::new(&data_folder);
        let mining_effects = Arc::new(MiningEffects::load(&directory.join("mining.toml"))?);
        let manager = Self::open(directory)?;
        let boss_bars = Arc::clone(&manager.boss_bars);
        let storage = Arc::clone(&manager.storage);
        *MANAGER.lock().expect("skill manager lock") = Some(manager);
        storage.schedule_autosave(context);
        Ok((mining_effects, boss_bars))
    }

    pub(crate) fn shutdown() -> Result<()> {
        let manager = MANAGER.lock().expect("skill manager lock").take();
        if let Some(manager) = manager {
            manager.boss_bars.shutdown();
            manager.storage.shutdown()?;
        }
        Ok(())
    }
    fn open(directory: &Path) -> Result<Self> {
        let boss_bars = Arc::new(BossBarManager::load(&directory.join("bossbar.toml"))?);
        let config = ExperienceConfig::load(&directory.join("experience.toml"))?;
        let storage = Arc::new(SkillStorage::open(directory)?);
        Ok(Self {
            boss_bars,
            config,
            storage,
        })
    }

    fn update_experience(&self, key: SkillKey, experience: impl FnOnce(u32) -> u64) -> SkillInfo {
        let previous = self.storage.progress(key);
        let progress = self.storage.update(key, |current| {
            self.config
                .advance(current.level, experience(current.experience))
        });
        let info = self.info(key, progress);
        if progress != previous {
            self.boss_bars.experience_updated(key.1, info);
        }
        info
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

fn with_manager<T>(use_manager: impl FnOnce(&SkillManager) -> T) -> Result<T> {
    MANAGER
        .lock()
        .expect("skill manager lock")
        .as_ref()
        .map(use_manager)
        .ok_or_else(|| "Skill manager is not loaded".into())
}

#[derive(Deserialize)]
#[serde(default)]
struct ExperienceConfig {
    base_experience: u32,
    level_multiplier: f64,
}

impl Default for ExperienceConfig {
    fn default() -> Self {
        Self {
            base_experience: 100,
            level_multiplier: 1.1,
        }
    }
}

impl ExperienceConfig {
    fn load(path: &Path) -> Result<Self> {
        let config: Self = match fs::read_to_string(path) {
            Ok(contents) => toml::from_str(&contents)
                .map_err(|error| format!("Cannot parse {}: {error}", path.display()))?,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                let defaults = Self::default();
                let contents = format!(
                    "base_experience = {}\nlevel_multiplier = {}\n",
                    defaults.base_experience, defaults.level_multiplier
                );
                fs::write(path, contents)
                    .map_err(|error| format!("Cannot create {}: {error}", path.display()))?;
                defaults
            }
            Err(error) => return Err(format!("Cannot read {}: {error}", path.display())),
        };
        if config.base_experience == 0
            || !config.level_multiplier.is_finite()
            || config.level_multiplier < 1.0
        {
            return Err(format!(
                "{} requires a positive base_experience and a finite level_multiplier of at least 1.0",
                path.display()
            ));
        }
        Ok(config)
    }

    fn required_experience(&self, level: u32) -> u32 {
        (f64::from(self.base_experience) * self.level_multiplier.powf(f64::from(level - 1))).round()
            as u32
    }

    fn advance(&self, mut level: u32, mut experience: u64) -> Progress {
        while level < u32::MAX {
            let required = self.required_experience(level);
            let affordable_levels =
                (experience / u64::from(required)).min(u64::from(u32::MAX - level)) as u32;
            if affordable_levels == 0 {
                break;
            }

            // Rounding can give many consecutive levels the same cost.
            let mut low = 1;
            let mut high = affordable_levels;
            while low < high {
                let mid = low + (high - low).div_ceil(2);
                if self.required_experience(level + (mid - 1)) == required {
                    low = mid;
                } else {
                    high = mid - 1;
                }
            }
            level += low;
            experience -= u64::from(low) * u64::from(required);
        }
        Progress {
            level,
            experience: experience.min(u64::from(u32::MAX)) as u32,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn mining_drop_probabilities_give_triple_priority() {
        for (level, normal, double, triple) in [
            (0, 100, 0, 0),
            (1, 80, 15, 5),
            (4, 20, 60, 20),
            (5, 0, 75, 25),
            (6, 0, 70, 30),
            (19, 0, 5, 95),
            (20, 0, 0, 100),
            (u32::MAX, 0, 0, 100),
        ] {
            assert_eq!(
                DropProbabilities::for_mining_level(level),
                DropProbabilities {
                    normal,
                    double,
                    triple,
                },
            );
        }
    }

    #[test]
    fn persists_level_and_experience_on_flush_and_shutdown() {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let directory = std::env::temp_dir().join(format!(
            "rummo-skills-{}-{}",
            std::process::id(),
            timestamp.as_nanos()
        ));
        fs::create_dir(&directory).unwrap();
        let manager = SkillManager::open(&directory).unwrap();
        let key = (1, SkillType::Mining);

        let skill = manager.update_experience(key, |_| 215);
        assert_eq!((skill.level, skill.experience), (3, 5));
        assert_eq!(skill.required_experience, Some(121));
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

        manager.update_experience(key, |_| 121);
        *MANAGER.lock().unwrap() = Some(manager);
        SkillManager::shutdown().unwrap();
        let saved = SkillStorage::open(&directory).unwrap().progress(key);
        assert_eq!(
            saved,
            Progress {
                level: 4,
                experience: 0
            }
        );

        fs::remove_file(directory.join("skills.redb")).unwrap();
        fs::remove_file(directory.join("experience.toml")).unwrap();
        fs::remove_file(directory.join("bossbar.toml")).unwrap();
        fs::remove_dir(directory).unwrap();
    }
}
