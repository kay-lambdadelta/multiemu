use audio::Chip8Audio;
use display::Chip8Display;
use multiemu_config::Environment;
use multiemu_definition_misc::memory::standard::{
    StandardMemory, StandardMemoryConfig, StandardMemoryInitialContents,
};
use multiemu_machine::{builder::MachineBuilder, memory::AddressSpaceId};
use multiemu_rom::{
    id::RomId,
    manager::RomManager,
    system::{GameSystem, OtherSystem},
};
use processor::Chip8Processor;
pub use processor::decoder::Chip8InstructionDecoder;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    sync::{Arc, RwLock},
};
use timer::Chip8Timer;

mod audio;
mod display;
mod processor;
mod timer;

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
pub enum Chip8Kind {
    Chip8,
    Chip8x,
    Chip48,
    SuperChip8,
    XoChip,
}

#[rustfmt::skip]
const CHIP8_FONT: [[u8; 5]; 16] = [
    [
        0b11110000,
        0b10010000,
        0b10010000,
        0b10010000,
        0b11110000,
    ],
    [
        0b00100000,
        0b01100000,
        0b00100000,
        0b00100000,
        0b01110000,
    ],
    [
        0b11110000,
        0b00010000,
        0b11110000,
        0b10000000,
        0b11110000,
    ],
    [
        0b11100000,
        0b00100000,
        0b11100000,
        0b00100000,
        0b11100000,
    ],
    [
        0b10010000,
        0b10010000,
        0b11110000,
        0b00010000,
        0b00010000,
    ],
    [
        0b11110000,
        0b10000000,
        0b11110000,
        0b00010000,
        0b11110000,
    ],
    [
        0b11110000,
        0b10000000,
        0b11110000,
        0b10010000,
        0b11110000,
    ],
    [
        0b11110000,
        0b00010000,
        0b00010000,
        0b00010000,
        0b00010000,
    ],
    [
        0b11110000,
        0b10010000,
        0b11110000,
        0b10010000,
        0b11110000,
    ],
    [
        0b11110000,
        0b10010000,
        0b11110000,
        0b00010000,
        0b11110000,
    ],
    [
        0b11110000,
        0b10010000,
        0b11110000,
        0b10010000,
        0b10010000,
    ],
    [
        0b11110000,
        0b10010000,
        0b11100000,
        0b10010000,
        0b11110000,
    ],
    [
        0b11110000,
        0b10000000,
        0b10000000,
        0b10000000,
        0b11110000,
    ],
    [
        0b11100000,
        0b10010000,
        0b10010000,
        0b10010000,
        0b11100000,
    ],
    [
        0b11110000,
        0b10000000,
        0b11110000,
        0b10000000,
        0b11110000,
    ],
    [
        0b11110000,
        0b10000000,
        0b11110000,
        0b10000000, 
        0b10000000,
    ],
];

const CPU_ADDRESS_SPACE: AddressSpaceId = AddressSpaceId::new(0);

pub fn manifest(
    user_specified_roms: Vec<RomId>,
    rom_manager: Arc<RomManager>,
    environment: Arc<RwLock<Environment>>,
) -> MachineBuilder {
    let machine = MachineBuilder::new(
        GameSystem::Other(OtherSystem::Chip8),
        rom_manager.clone(),
        environment.clone(),
    );

    let machine = machine.insert_address_space(CPU_ADDRESS_SPACE, 12);
    let machine = machine.insert_default_component::<Chip8Timer>("timer");
    let machine = machine.insert_default_component::<Chip8Audio>("audio");
    let machine = machine.insert_default_component::<Chip8Display>("display");
    let machine = machine.insert_default_component::<Chip8Processor>("cpu");
    let machine = machine.insert_component::<StandardMemory>(
        "workram",
        StandardMemoryConfig {
            readable: true,
            writable: true,
            max_word_size: 2,
            assigned_range: 0x000..=0xfff,
            assigned_address_space: CPU_ADDRESS_SPACE,
            initial_contents: vec![
                StandardMemoryInitialContents::Array {
                    value: Cow::Borrowed(bytemuck::cast_slice(&CHIP8_FONT)),
                    offset: 0x000,
                },
                StandardMemoryInitialContents::Rom {
                    rom_id: user_specified_roms[0],
                    offset: 0x200,
                },
            ],
        },
    );

    machine
}
