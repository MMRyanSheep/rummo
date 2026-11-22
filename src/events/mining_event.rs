use std::collections::HashMap;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use pumpkin_plugin_api::events::{BlockBreakEvent, EventData, EventHandler, EventPriority};
use pumpkin_plugin_api::{Context, Result, Server};

use crate::skills::skill_manager::{SkillManager, SkillType};

const DEFAULT_EXPERIENCE: &str = include_str!("../../config/mining_experience.toml");

pub(crate) struct MiningEvent {
    experience: HashMap<String, u32>,
}

impl MiningEvent {
    pub(crate) fn load(context: &Context) -> Result<Self> {
        let path = Path::new(&context.get_data_folder()).join("mining_experience.toml");
        let contents = match fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                fs::write(&path, DEFAULT_EXPERIENCE)
                    .map_err(|error| format!("Cannot create {}: {error}", path.display()))?;
                DEFAULT_EXPERIENCE.to_owned()
            }
            Err(error) => return Err(format!("Cannot read {}: {error}", path.display())),
        };
        let rewards: HashMap<String, u32> = toml::from_str(&contents)
            .map_err(|error| format!("Cannot parse {}: {error}", path.display()))?;
        let mut experience = HashMap::with_capacity(rewards.len());
        for (block, exp) in rewards {
            let id = block_id(&block);
            if experience.insert(id.to_owned(), exp).is_some() {
                return Err(format!(
                    "Duplicate mining experience for {id} in {}",
                    path.display()
                ));
            }
        }
        Ok(Self { experience })
    }

    pub(crate) fn register(self, context: &Context) -> Result<()> {
        context.register_event_handler(self, EventPriority::Normal, false)?;
        Ok(())
    }
}

impl EventHandler<BlockBreakEvent> for MiningEvent {
    fn handle(
        &self,
        _server: Server,
        data: EventData<BlockBreakEvent>,
    ) -> EventData<BlockBreakEvent> {
        if data.cancelled {
            return data;
        }
        let Some(player) = data.player.as_ref() else {
            return data;
        };
        let Some(&exp) = self.experience.get(block_id(&data.block)) else {
            return data;
        };

        if exp > 0
            && let Err(error) = SkillManager::addExp(player, SkillType::Mining, exp)
        {
            tracing::error!(block = %data.block, %error, "Failed to add mining experience");
        }
        return data
    }
}

fn block_id(block: &str) -> &str {
    block.strip_prefix("minecraft:").unwrap_or(block)
}
