use crate::{ComponentName, ComponentSnapshot};
use multiemu_rom::RomId;
use redb::{Database, ReadOnlyTable, ReadTransaction, TableDefinition};
use std::{fs::create_dir_all, path::PathBuf};

const SNAPSHOT_TABLE: TableDefinition<'static, ComponentName, ComponentSnapshot> =
    TableDefinition::new("snapshot");

#[derive(Debug)]
pub struct SnapshotManager {
    snapshot_files: scc::HashCache<(RomId, u8), Database>,
    snapshot_directory: Option<PathBuf>,
}

impl SnapshotManager {
    pub fn new(snapshot_directory: Option<PathBuf>) -> Self {
        Self {
            snapshot_files: scc::HashCache::with_capacity(0, 8),
            snapshot_directory,
        }
    }

    pub fn open(
        &self,
        slot: u8,
        rom_id: RomId,
    ) -> Result<
        (
            ReadTransaction,
            ReadOnlyTable<ComponentName, ComponentSnapshot>,
        ),
        Box<dyn std::error::Error>,
    > {
        let entry = match self.snapshot_files.entry((rom_id, slot)) {
            scc::hash_cache::Entry::Occupied(occupied_entry) => occupied_entry,
            scc::hash_cache::Entry::Vacant(vacant_entry) => {
                let path = self
                    .snapshot_directory
                    .as_ref()
                    .unwrap()
                    .join(slot.to_string());
                create_dir_all(&path)?;

                let db = Database::open(path.join(rom_id.to_string()))?;
                let (_, entry) = vacant_entry.put_entry(db);

                entry
            }
        };

        let transaction = entry.begin_read()?;
        let read_only_table = transaction.open_table(SNAPSHOT_TABLE)?;

        Ok((transaction, read_only_table))
    }
}
