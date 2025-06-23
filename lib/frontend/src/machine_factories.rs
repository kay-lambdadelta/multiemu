use multiemu_rom::{GameSystem, RomId, RomManager};
use multiemu_runtime::{MachineFactory, builder::MachineBuilder, platform::Platform};
use num::rational::Ratio;
use std::{collections::HashMap, sync::Arc};

#[derive(Debug)]
pub struct MachineFactories<P: Platform>(HashMap<GameSystem, Box<dyn MachineFactory<P>>>);

impl<P: Platform> MachineFactories<P> {
    pub fn insert_factory<M: MachineFactory<P> + Default>(&mut self, system: GameSystem) {
        self.0.insert(system, Box::new(M::default()));
    }

    pub fn construct_machine(
        &self,
        system: GameSystem,
        user_specified_roms: Vec<RomId>,
        rom_manager: Arc<RomManager>,
        sample_rate: Ratio<u32>,
        main_thread_executor: Arc<P::MainThreadExecutor>,
    ) -> MachineBuilder<P> {
        self.0
            .get(&system)
            .unwrap_or_else(|| panic!("No factory for system {:?}", system))
            .construct(
                user_specified_roms,
                rom_manager,
                sample_rate,
                main_thread_executor,
            )
    }
}

impl<P: Platform> Default for MachineFactories<P> {
    fn default() -> Self {
        Self(Default::default())
    }
}
