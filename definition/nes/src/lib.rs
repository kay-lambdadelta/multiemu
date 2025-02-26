pub use cartridge::ines::INes;
use cartridge::{NesCartridge, NesCartridgeConfig};
use multiemu_definition_misc::memory::mirror::{MirrorMemory, MirrorMemoryConfig};
use multiemu_definition_misc::memory::standard::{
    StandardMemory, StandardMemoryConfig, StandardMemoryInitialContents,
};
use multiemu_macros::manifest;
use multiemu_rom::system::{GameSystem, NintendoSystem};
use ppu::NesPpu;
use rangemap::RangeMap;

mod cartridge;
mod ppu;

enum Region {
    Ntsc,
    Pal,
}

manifest! {
    machine: GameSystem::Nintendo(NintendoSystem::NintendoEntertainmentSystem),
    address_spaces: {
        CPU_ADDRESS_SPACE: 16,
        PPU_ADDRESS_SPACE: 16
    },
    components: {
        NesCartridge("cartridge"): NesCartridgeConfig {
            rom: user_specified_roms[0]
        },
        StandardMemory("workram"): StandardMemoryConfig {
            readable: true,
            writable: true,
            max_word_size: 2,
            assigned_range: 0x0000..0x0800,
            assigned_address_space: CPU_ADDRESS_SPACE,
            initial_contents: vec![StandardMemoryInitialContents::Random],
        },
        MirrorMemory("workram-mirror"): MirrorMemoryConfig {
            readable: true,
            writable: true,
            assigned_ranges: RangeMap::from_iter([
                (0x0800..0x1000, 0x0000),
                (0x1000..0x1800, 0x0000),
                (0x1800..0x2000, 0x0000),
            ]),
            assigned_address_space: CPU_ADDRESS_SPACE,
        },
        NesPpu("ppu"): Default::default(),
    }
}
