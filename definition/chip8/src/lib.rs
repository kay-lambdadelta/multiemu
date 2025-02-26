use audio::Chip8Audio;
use display::Chip8Display;
use multiemu_definition_misc::memory::standard::{
    StandardMemory, StandardMemoryConfig, StandardMemoryInitialContents,
};
use multiemu_macros::manifest;
use multiemu_rom::system::{GameSystem, OtherSystem};
use processor::Chip8Processor;
pub use processor::decoder::Chip8InstructionDecoder;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
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

manifest! {
    machine: GameSystem::Other(OtherSystem::Chip8),
    address_spaces: {
        ADDRESS_SPACE_ID: 12,
    },
    components: {
        Chip8Timer("timer"): Default::default(),
        Chip8Audio("audio"): Default::default(),
        Chip8Display("display"): Default::default(),
        Chip8Processor("cpu"): Default::default(),
        StandardMemory("workram"): StandardMemoryConfig {
            readable: true,
            writable: true,
            max_word_size: 2,
            assigned_range: 0x000..0x1000,
            assigned_address_space: ADDRESS_SPACE_ID,
            initial_contents: vec![
                StandardMemoryInitialContents::Array {
                    value: Cow::Borrowed(bytemuck::cast_slice(&CHIP8_FONT)),
                    offset: 0x000,
                },
                StandardMemoryInitialContents::Rom {
                    rom_id: user_specified_roms[0],
                    offset: 0x200,
                }
            ],
        },
    }
}
