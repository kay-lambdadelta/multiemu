use multiemu_runtime::{
    machine::{MachineFactory, builder::MachineBuilder},
    platform::Platform,
    program::MachineId,
};
use std::{collections::HashMap, fmt::Debug};

/// Factory storage for frontend machine generation automation
pub struct MachineFactories<P: Platform>(HashMap<MachineId, Box<dyn MachineFactory<P>>>);

impl<P: Platform> Debug for MachineFactories<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MachineFactories").finish()
    }
}

impl<P: Platform> MachineFactories<P> {
    /// Register a factory
    pub fn insert_factory<M: MachineFactory<P> + Default>(&mut self, system: MachineId) {
        self.0.insert(system, Box::new(M::default()));
    }

    /// Spit out a machine based upon the factories
    pub fn construct_machine(&self, machine_builder: MachineBuilder<P>) -> MachineBuilder<P> {
        let system = machine_builder.machine_id().unwrap();

        self.0
            .get(&system)
            .unwrap_or_else(|| panic!("No factory for system {system:?}"))
            .construct(machine_builder)
    }
}

impl<P: Platform> Default for MachineFactories<P> {
    fn default() -> Self {
        Self(Default::default())
    }
}
