//! Skill persistence for existing redb databases.

use std::path::{Path, PathBuf};

use pumpkin_plugin_api::Result;
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};

use crate::skills::skill_manager::SkillType;

use super::{Progress, SkillRows};

// (player_uuid, skill_id) -> (level, experience)
const SKILLS: TableDefinition<(u128, &str), (u32, u32)> = TableDefinition::new("skills");

pub(super) struct SkillDatabase {
    path: PathBuf,
}

impl SkillDatabase {
    pub(super) fn open(directory: &Path) -> Result<(Self, SkillRows)> {
        let path = directory.join("skills.redb");
        let rows = Self::load_rows(&path)
            .map_err(|error| format!("Cannot initialize {}: {error}", path.display()))?;
        Ok((Self { path }, rows))
    }

    fn load_rows(path: &Path) -> Result<SkillRows, redb::Error> {
        let database = Database::create(path)?;
        let transaction = database.begin_write()?;
        transaction.open_table(SKILLS)?;
        transaction.commit()?;

        let transaction = database.begin_read()?;
        let table = transaction.open_table(SKILLS)?;
        let mut rows = SkillRows::new();
        for row in table.iter()? {
            let (key, value) = row?;
            let (uuid, id) = key.value();
            if let Some(skill_type) = SkillType::from_id(id) {
                let (level, experience) = value.value();
                rows.insert((uuid, skill_type), Progress { level, experience });
            }
        }
        Ok(rows)
    }

    pub(super) fn commit(&self, rows: &SkillRows) -> Result<()> {
        if rows.is_empty() {
            return Ok(());
        }
        self.write_rows(rows)
            .map_err(|error| format!("Cannot save {}: {error}", self.path.display()))
    }

    fn write_rows(&self, rows: &SkillRows) -> Result<(), redb::Error> {
        let database = Database::open(&self.path)?;
        let transaction = database.begin_write()?;
        {
            let mut table = transaction.open_table(SKILLS)?;
            for (&(uuid, skill_type), progress) in rows {
                table.insert(
                    (uuid, skill_type.id()),
                    (progress.level, progress.experience),
                )?;
            }
        }
        transaction.commit()?;
        Ok(())
    }
}
