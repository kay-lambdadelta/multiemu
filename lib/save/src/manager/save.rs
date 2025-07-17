use crate::{ComponentName, ComponentSave};
use multiemu_rom::RomId;
use redb::{Database, ReadOnlyTable, ReadTransaction, TableDefinition};
use std::{fs::create_dir_all, path::PathBuf};

const SAVE_TABLE: TableDefinition<'static, ComponentName, ComponentSave> =
    TableDefinition::new("save");

#[derive(Debug)]
pub struct SaveManager {
    save_files: scc::HashCache<RomId, Database>,
    save_directory: Option<PathBuf>,
}

impl SaveManager {
    pub fn new(save_directory: Option<PathBuf>) -> Self {
        Self {
            save_files: scc::HashCache::with_capacity(0, 4),
            save_directory,
        }
    }

    pub fn open(
        &self,
        rom_id: RomId,
    ) -> Result<
        (ReadTransaction, ReadOnlyTable<ComponentName, ComponentSave>),
        Box<dyn std::error::Error>,
    > {
        let entry = match self.save_files.entry(rom_id) {
            scc::hash_cache::Entry::Occupied(occupied_entry) => occupied_entry,
            scc::hash_cache::Entry::Vacant(vacant_entry) => {
                let path = self.save_directory.as_ref().unwrap();
                create_dir_all(&path)?;

                let db = Database::open(path.join(rom_id.to_string()))?;
                let (_, entry) = vacant_entry.put_entry(db);

                entry
            }
        };

        let transaction = entry.begin_read()?;
        let read_only_table = transaction.open_table(SAVE_TABLE)?;

        Ok((transaction, read_only_table))
    }
}
