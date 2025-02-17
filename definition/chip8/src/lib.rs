use audio::Chip8Audio;
use display::{Chip8Display, Chip8DisplayConfig};
use multiemu_config::Environment;
use multiemu_definition_misc::memory::standard::{
    StandardMemory, StandardMemoryConfig, StandardMemoryInitialContents,
};
use multiemu_machine::builder::MachineBuilder;
use multiemu_machine::memory::AddressSpaceId;
use multiemu_machine::Machine;
use multiemu_rom::id::RomId;
use multiemu_rom::manager::RomManager;
use multiemu_rom::system::{GameSystem, OtherSystem};
use num::rational::Ratio;
use processor::{Chip8Processor, Chip8ProcessorConfig};
use std::borrow::Cow;
use std::sync::{Arc, RwLock};
use timer::Chip8Timer;

mod audio;
mod display;
mod processor;
mod timer;

pub use processor::decoder::Chip8InstructionDecoder;

const CHIP8_ADDRESS_SPACE_ID: AddressSpaceId = AddressSpaceId::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

pub fn build_machine(
    user_specified_roms: Vec<RomId>,
    rom_manager: Arc<RomManager>,
    environment: Arc<RwLock<Environment>>,
) -> MachineBuilder {
    let machine = Machine::build(
        GameSystem::Other(OtherSystem::Chip8),
        rom_manager,
        environment,
    );
    let machine = machine.insert_bus(CHIP8_ADDRESS_SPACE_ID, 12);

    let (machine, display_component_id) =
        machine.insert_component::<Chip8Display>(Chip8DisplayConfig {
            kind: Chip8Kind::Chip8,
        });
    let (machine, timer_component_id) = machine.insert_default_component::<Chip8Timer>();
    let (machine, audio_component_id) = machine.insert_default_component::<Chip8Audio>();

    let (machine, _) = machine.insert_component::<Chip8Processor>(Chip8ProcessorConfig {
        frequency: Ratio::from_integer(1000),
        kind: Chip8Kind::Chip8,
        display_component_id,
        timer_component_id,
        audio_component_id,
    });

    let (machine, _) = machine.insert_component::<StandardMemory>(StandardMemoryConfig {
        readable: true,
        writable: true,
        max_word_size: 2,
        assigned_range: 0x000..0x200,
        assigned_address_space: CHIP8_ADDRESS_SPACE_ID,
        initial_contents: StandardMemoryInitialContents::Array {
            value: Cow::Borrowed(bytemuck::cast_slice(&CHIP8_FONT)),
            offset: 0x000,
        },
    });

    let (machine, _) = machine.insert_component::<StandardMemory>(StandardMemoryConfig {
        readable: true,
        writable: true,
        max_word_size: 2,
        assigned_range: 0x200..0x1000,
        assigned_address_space: CHIP8_ADDRESS_SPACE_ID,
        initial_contents: StandardMemoryInitialContents::Rom {
            rom_id: user_specified_roms[0],
            offset: 0x200,
        },
    });

    machine
}
