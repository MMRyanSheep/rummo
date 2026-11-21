mod commands;
#[path = "skillManager.rs"]
pub mod skill_manager;
mod storage;

use pumpkin_plugin_api::permissions::FS_WRITE_DATA;
use pumpkin_plugin_api::{Context, Plugin, PluginMetadata, Result};
use skill_manager::SkillManager;
use tracing::info;

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
        commands::register(&context)?;
        SkillManager::initialize(&context)?;
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
