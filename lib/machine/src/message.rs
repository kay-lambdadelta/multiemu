use crate::component::ComponentId;
use crate::memory::memory_translation_table::MemoryTranslationTable;
use redb::Database;
use std::sync::Arc;

pub enum MachineMessage {
    SnapshotMachine { database: Database },
    SetMemoryTranslationTable { mtt: Arc<MemoryTranslationTable> },
    ConstructComponent { component_id: ComponentId },
    FinalizeExternalMachine,
    RunComponent { component: ComponentId, period: u64 },
    Shutdown,
}
