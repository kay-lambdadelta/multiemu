use cartridge::{Atari2600Cartridge, Atari2600CartridgeConfig};
use gamepad::joystick::{Atari2600Joystick, Atari2600JoystickConfig};
use multiemu_config::Environment;
use multiemu_definition_m6502::{M6502, M6502Config, M6502Kind};
use multiemu_definition_misc::m6532_riot::{M6532Riot, M6532RiotConfig};
use multiemu_machine::builder::MachineBuilder;
use multiemu_machine::display::shader::ShaderCache;
use multiemu_machine::memory::AddressSpaceId;
use multiemu_rom::id::RomId;
use multiemu_rom::manager::RomManager;
use multiemu_rom::system::{AtariSystem, GameSystem};
use num::rational::Ratio;
use std::sync::{Arc, RwLock};
use tia::Tia;

mod cartridge;
mod gamepad;
mod tia;

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

    let machine = machine.insert_address_space(CPU_ADDRESS_SPACE, 16);

    let cpu_frequency = Ratio::from_integer(1190000);

    machine
        .insert_component::<M6502>(
            "processor",
            M6502Config {
                frequency: cpu_frequency,
                kind: M6502Kind::M6507,
                assigned_address_space: CPU_ADDRESS_SPACE,
            },
        )
        .insert_component::<M6532Riot>(
            "m6532_riot",
            M6532RiotConfig {
                frequency: cpu_frequency,
                ram_assigned_address: 0x080,
                registers_assigned_address: 0x280,
                assigned_address_space: CPU_ADDRESS_SPACE,
            },
        )
        .insert_component::<Atari2600Joystick>(
            "joystick",
            Atari2600JoystickConfig {
                m6532_riot: "m6532_riot".into(),
            },
        )
        .insert_component::<Atari2600Cartridge>(
            "cartridge",
            Atari2600CartridgeConfig {
                rom: user_specified_roms[0],
            },
        )
        .insert_default_component::<Tia>("tia")
}
