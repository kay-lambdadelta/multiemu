use crate::{
    machine::{MachineFactory, builder::MachineBuilder},
    platform::Platform,
    rom::System,
};
use std::{collections::HashMap, fmt::Debug};

pub struct MachineFactories<P: Platform>(HashMap<System, Box<dyn MachineFactory<P>>>);

impl<P: Platform> Debug for MachineFactories<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MachineFactories").finish()
    }
}

impl<P: Platform> MachineFactories<P> {
    pub fn insert_factory<M: MachineFactory<P> + Default>(&mut self, system: System) {
        self.0.insert(system, Box::new(M::default()));
    }

    pub fn construct_machine(&self, machine_builder: MachineBuilder<P>) -> MachineBuilder<P> {
        let system = machine_builder.system().unwrap();

        self.0
            .get(&system)
            .unwrap_or_else(|| panic!("No factory for system {:?}", system))
            .construct(machine_builder)
    }
}

impl<P: Platform> Default for MachineFactories<P> {
    fn default() -> Self {
        Self(Default::default())
    }
}
