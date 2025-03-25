use std::sync::{Arc, RwLock};

pub use cartridge::ines::INes;
use cartridge::ines::TimingMode;
use cartridge::{NesCartridge, NesCartridgeConfig};
use multiemu_config::Environment;
use multiemu_definition_m6502::{M6502, M6502Config, M6502Kind};
use multiemu_definition_misc::memory::mirror::{MirrorMemory, MirrorMemoryConfig};
use multiemu_definition_misc::memory::standard::{
    StandardMemory, StandardMemoryConfig, StandardMemoryInitialContents,
};
use multiemu_machine::builder::MachineBuilder;
use multiemu_machine::display::shader::ShaderCache;
use multiemu_machine::memory::AddressSpaceId;
use multiemu_rom::id::RomId;
use multiemu_rom::manager::RomManager;
use multiemu_rom::system::{GameSystem, NintendoSystem};
use num::rational::Ratio;
use ppu::NesPpu;
use rangemap::RangeInclusiveMap;

mod apu;
mod cartridge;
mod ppu;

const CPU_ADDRESS_SPACE: AddressSpaceId = AddressSpaceId::new(0);
const PPU_ADDRESS_SPACE: AddressSpaceId = AddressSpaceId::new(1);

pub fn manifest(
    user_specified_roms: Vec<RomId>,
    rom_manager: Arc<RomManager>,
    environment: Arc<RwLock<Environment>>,
    shader_cache: Arc<ShaderCache>,
) -> MachineBuilder {
    let machine = multiemu_machine::builder::MachineBuilder::new(
        GameSystem::Nintendo(NintendoSystem::NintendoEntertainmentSystem),
        rom_manager.clone(),
        environment.clone(),
        shader_cache.clone(),
    );

    let machine = machine.insert_address_space(CPU_ADDRESS_SPACE, 16);
    let machine = machine.insert_address_space(PPU_ADDRESS_SPACE, 16);

    let machine = machine.insert_component::<NesCartridge>(
        "cartridge",
        NesCartridgeConfig {
            rom: user_specified_roms[0],
        },
    );

    // Grab the timing mode
    let mut timing_mode = TimingMode::Ntsc;
    machine
        .component_store()
        .interact_by_name_local::<NesCartridge>("cartridge", |cart| {
            timing_mode = cart.rom().timing_mode
        });

    let processor_frequency = Ratio::from_integer(match timing_mode {
        TimingMode::Ntsc => 1789773,
        TimingMode::Pal => 2097152,
        TimingMode::Multi => 1789773,
        TimingMode::Dendy => 1773448,
    });

    let machine = machine.insert_component::<StandardMemory>(
        "workram",
        StandardMemoryConfig {
            readable: true,
            writable: true,
            max_word_size: 2,
            assigned_range: 0x0000..=0x07ff,
            assigned_address_space: CPU_ADDRESS_SPACE,
            initial_contents: vec![StandardMemoryInitialContents::Random],
        },
    );
    let machine = machine.insert_component::<MirrorMemory>(
        "workram-mirror",
        MirrorMemoryConfig {
            readable: true,
            writable: true,
            assigned_ranges: RangeInclusiveMap::from_iter([
                (0x0800..=0x0fff, 0x0000),
                (0x1000..=0x17ff, 0x0000),
                (0x1800..=0x1fff, 0x0000),
            ]),
            assigned_address_space: CPU_ADDRESS_SPACE,
        },
    );

    let machine = machine.insert_default_component::<NesPpu>("ppu");

    machine.insert_component::<M6502>(
        "processor",
        M6502Config {
            frequency: processor_frequency,
            assigned_address_space: CPU_ADDRESS_SPACE,
            kind: M6502Kind::R2A0x,
        },
    )
}
