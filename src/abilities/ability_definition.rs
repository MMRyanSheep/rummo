use std::time::Duration;

use serde::{Deserialize, Deserializer, de};

use crate::skills::skill_manager::SkillType;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "Ability effects do not use these settings yet")
)]
pub(crate) struct AbilityDefinition {
    #[serde(deserialize_with = "deserialize_parent_skill")]
    pub(crate) parent_skill: SkillType,
    pub(crate) unlock_level: u32,
    #[serde(deserialize_with = "deserialize_duration")]
    pub(crate) base_cooldown: Duration,
    #[serde(deserialize_with = "deserialize_duration")]
    pub(crate) cooldown_per_level: Duration,
    #[serde(deserialize_with = "deserialize_duration")]
    pub(crate) min_cooldown: Duration,
    #[serde(deserialize_with = "deserialize_duration")]
    pub(crate) duration: Duration,
}

fn deserialize_parent_skill<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<SkillType, D::Error> {
    let id = String::deserialize(deserializer)?;
    SkillType::from_id(&id).ok_or_else(|| de::Error::custom(format!("Unknown parent_skill `{id}`")))
}

fn deserialize_duration<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Duration, D::Error> {
    let seconds = f64::deserialize(deserializer)?;
    Duration::try_from_secs_f64(seconds).map_err(de::Error::custom)
}
