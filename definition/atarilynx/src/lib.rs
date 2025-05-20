use multiemu_config::Environment;
use multiemu_definition_misc::memory::rom::RomMemoryConfig;
use multiemu_machine::{
    MachineFactory,
    builder::MachineBuilder,
    display::{backend::RenderApi, shader::ShaderCache},
};
use multiemu_rom::{
    id::RomId,
    manager::RomManager,
    system::{AtariSystem, GameSystem},
};
use std::{
    str::FromStr,
    sync::{Arc, RwLock},
};

#[derive(Debug, Default)]
pub struct AtariLynx;

impl<R: RenderApi> MachineFactory<R> for AtariLynx {
    fn construct(
        &self,
        _user_specified_roms: Vec<RomId>,
        rom_manager: Arc<RomManager>,
        environment: Arc<RwLock<Environment>>,
        shader_cache: ShaderCache,
    ) -> MachineBuilder<R> {
        let machine = MachineBuilder::new(
            GameSystem::Atari(AtariSystem::Lynx),
            rom_manager.clone(),
            environment,
            shader_cache,
        );

        let (machine, cpu_address_space) = machine.insert_address_space("cpu", 16);

        let (machine, _) = machine.insert_component(
            "bootstrap",
            RomMemoryConfig {
                // "[BIOS] Atari Lynx (World).lyx"
                rom: RomId::from_str("e4ed47fae31693e016b081c6bda48da5b70d7ccb").unwrap(),
                max_word_size: 1,
                assigned_range: 0xfe00..=0xffff,
                assigned_address_space: cpu_address_space,
            },
        );

        machine
    }
}
