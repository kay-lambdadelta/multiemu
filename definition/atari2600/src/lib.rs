use multiemu_config::Environment;
use multiemu_definition_m6502::{M6502, M6502Config, M6502Kind, M6502Registers};
use multiemu_machine::builder::MachineBuilder;
use multiemu_machine::display::shader::ShaderCache;
use multiemu_machine::memory::AddressSpaceId;
use multiemu_rom::id::RomId;
use multiemu_rom::manager::RomManager;
use multiemu_rom::system::{AtariSystem, GameSystem};
use num::rational::Ratio;
use std::sync::{Arc, RwLock};

mod cartridge;

const CPU_ADDRESS_SPACE: AddressSpaceId = AddressSpaceId::new(0);

pub fn manifest(
    user_specified_roms: Vec<RomId>,
    rom_manager: Arc<RomManager>,
    environment: Arc<RwLock<Environment>>,
    shader_cache: Arc<ShaderCache>,
) -> MachineBuilder {
    let machine = MachineBuilder::new(
        GameSystem::Atari(AtariSystem::Atari2600),
        rom_manager,
        environment,
        shader_cache,
    );

    let machine = machine.insert_address_space(CPU_ADDRESS_SPACE, 13);

    machine.insert_component::<M6502>(
        "processor",
        M6502Config {
            assigned_address_space: CPU_ADDRESS_SPACE,
            frequency: Ratio::from_integer(1190000),
            kind: M6502Kind::M6507,
            initial_state: M6502Registers::default(),
        },
    )
}
