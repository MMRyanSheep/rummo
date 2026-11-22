mod cache;
mod sqlite;

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use pumpkin_plugin_api::scheduler::SchedulerExt;
use pumpkin_plugin_api::{Context, Result};

use crate::skills::skill_manager::SkillType;

use self::cache::SkillCache;
use self::sqlite::SkillDatabase;

const FLUSH_INTERVAL_TICKS: u64 = 20 * 30;

pub(crate) type SkillKey = (u128, SkillType);
type SkillRows = HashMap<SkillKey, Progress>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Progress {
    pub(crate) level: u32,
    pub(crate) experience: u32,
}

impl Default for Progress {
    fn default() -> Self {
        Self {
            level: 1,
            experience: 0,
        }
    }
}

pub(crate) struct SkillStorage {
    database: SkillDatabase,
    cache: Mutex<SkillCache>,
}

impl SkillStorage {
    pub(crate) fn open(directory: &Path) -> Result<Self> {
        let (database, rows) = SkillDatabase::open(directory)?;
        Ok(Self {
            database,
            cache: Mutex::new(SkillCache::new(rows)),
        })
    }

    pub(crate) fn progress(&self, key: SkillKey) -> Progress {
        self.cache.lock().expect("skill cache lock").progress(key)
    }

    pub(crate) fn update(
        &self,
        key: SkillKey,
        update: impl FnOnce(Progress) -> Progress,
    ) -> Progress {
        self.cache
            .lock()
            .expect("skill cache lock")
            .update(key, update)
    }

    pub(crate) fn schedule_autosave(self: &Arc<Self>, context: &Context) {
        let storage = Arc::downgrade(self);
        context.schedule_repeating_task(FLUSH_INTERVAL_TICKS, FLUSH_INTERVAL_TICKS, move |_| {
            if let Some(storage) = storage.upgrade()
                && let Err(error) = storage.flush_pending()
            {
                tracing::error!("Failed to save skill experience: {error}");
            }
        });
    }

    pub(crate) fn flush_pending(&self) -> Result<()> {
        let batch = self.cache.lock().expect("skill cache lock").begin_flush();
        let Some(batch) = batch else {
            return Ok(());
        };

        // Filesystem calls can yield to the host while the cache remains accessible.
        let result = self.database.commit(&batch);
        self.cache
            .lock()
            .expect("skill cache lock")
            .finish_flush(&batch, result.is_ok());
        result
    }

    pub(crate) fn shutdown(&self) -> Result<()> {
        // Pumpkin drains guest callbacks before invoking on_unload.
        let pending = self.cache.lock().expect("skill cache lock").take_pending();
        self.database.commit(&pending)
    }
}
