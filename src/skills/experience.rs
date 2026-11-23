use std::path::Path;

use pumpkin_plugin_api::Result;
use serde::Deserialize;

use crate::config;
use crate::storage::Progress;

const DEFAULT_EXPERIENCE: &str = include_str!("../../config/experience.toml");

#[derive(Deserialize)]
#[serde(default)]
pub(super) struct ExperienceConfig {
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
    pub(super) fn load(path: &Path) -> Result<Self> {
        let config: Self = config::load(path, DEFAULT_EXPERIENCE)?;
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

    pub(super) fn required_experience(&self, level: u32) -> u32 {
        (f64::from(self.base_experience) * self.level_multiplier.powf(f64::from(level - 1))).round()
            as u32
    }

    pub(super) fn progress_at_level(&self, level: u32, experience: u32) -> Progress {
        let experience = if level == u32::MAX {
            experience
        } else {
            experience.min(self.required_experience(level) - 1)
        };
        Progress { level, experience }
    }

    pub(super) fn advance(&self, mut level: u32, mut experience: u64) -> Progress {
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

    #[test]
    fn direct_level_changes_cap_experience_without_advancing() {
        let config = ExperienceConfig::default();

        assert_eq!(
            config.progress_at_level(1, 120),
            Progress {
                level: 1,
                experience: 99
            }
        );
        assert_eq!(
            config.progress_at_level(1, 100),
            Progress {
                level: 1,
                experience: 99
            }
        );
        assert_eq!(
            config.progress_at_level(3, 99),
            Progress {
                level: 3,
                experience: 99
            }
        );
        assert_eq!(
            config.progress_at_level(u32::MAX, u32::MAX),
            Progress {
                level: u32::MAX,
                experience: u32::MAX
            }
        );
    }
}
