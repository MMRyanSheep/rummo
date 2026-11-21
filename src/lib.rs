mod commands;

use pumpkin_plugin_api::{Context, Plugin, PluginMetadata, Result};
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
            permissions: Vec::new(),
        }
    }

    fn on_load(&self, context: Context) -> Result<()> {
        commands::register(&context)?;
        info!("rummo loaded");
        Ok(())
    }

    fn on_unload(&self, _context: Context) -> Result<()> {
        info!("rummo unloaded");
        Ok(())
    }
}

pumpkin_plugin_api::register_plugin!(Rummo);
