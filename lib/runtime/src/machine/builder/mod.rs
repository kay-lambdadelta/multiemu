use std::sync::Arc;

use crate::{
    memory::{AddressSpace, MemoryRemappingCommand},
    scheduler::{EventType, Period},
};

mod component;
mod machine;

pub use component::*;
pub use machine::*;

struct AddressSpaceInfo {
    address_space: Arc<AddressSpace>,
    memory_map_queue: Vec<MemoryRemappingCommand>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum SchedulerParticipation {
    /// The scheduler will make no attempt to time synchronize this component
    None,
    /// [`crate::component::Component::synchronize`] will only be called upon interaction
    OnDemand,
    /// [`crate::component::Component::synchronize`] will also be called when the scheduler advances time
    SchedulerDriven,
}

struct PartialEvent {
    ty: EventType,
    time: Period,
}
