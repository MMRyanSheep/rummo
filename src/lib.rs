mod commands;
mod events;
pub mod skills;
mod storage;
mod ui;

use events::{block_drop_item_event, mining_event::MiningEvent, player_join_event, player_leave_event};
use pumpkin_plugin_api::permissions::FS_WRITE_DATA;
use pumpkin_plugin_api::{Context, Plugin, PluginMetadata, Result};
use skills::{mining, skill_manager::SkillManager};
use tracing::info;
use ui::bossbar_scheduler;

pub use skills::skill_manager;

const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
const VERSION: &str = env!("CARGO_PKG_VERSION");

struct Rummo;

impl Plugin for Rummo {
    fn new() -> Self {
        Self
    }

    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: env!("CARGO_PKG_NAME").into(),
            version: VERSION.into(),
            authors: AUTHORS.split(':').map(str::to_owned).collect(),
            description: env!("CARGO_PKG_DESCRIPTION").into(),
            dependencies: Vec::new(),
            permissions: vec![FS_WRITE_DATA.into()],
        }
    }

    fn on_load(&self, context: Context) -> Result<()> {
        let mining_event = MiningEvent::load(&context)?;
        commands::register(&context)?;
        let (mining_effects, boss_bars) = SkillManager::initialize(&context)?;
        player_join_event::register(&context, mining_effects.clone(), boss_bars.clone())?;
        player_leave_event::register(&context, boss_bars.clone())?;
        mining::register(&context, mining_effects)?;
        mining_event.register(&context)?;
        block_drop_item_event::register(&context)?;
        bossbar_scheduler::register(&context, &boss_bars);
        info!("rummo loaded");
        Ok(())
    }

    fn on_unload(&self, _context: Context) -> Result<()> {
        SkillManager::shutdown()?;
        info!("rummo unloaded");
        Ok(())
    }
}

pumpkin_plugin_api::register_plugin!(Rummo);
