use crate::display::SupportedGraphicsApiChip8Display;
use audio::Chip8AudioConfig;
use display::Chip8DisplayConfig;
use font::CHIP8_FONT;
use multiemu::{
    machine::{MachineFactory, builder::MachineBuilder},
    platform::Platform,
};
use multiemu_definition_misc::memory::standard::{
    StandardMemoryConfig, StandardMemoryInitialContents,
};
use num::rational::Ratio;
use processor::Chip8ProcessorConfig;
pub use processor::decoder::Chip8InstructionDecoder;
use rangemap::RangeInclusiveMap;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use timer::Chip8TimerConfig;

mod audio;
mod display;
mod font;
mod processor;
mod timer;

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug, Default)]
pub enum Chip8Mode {
    #[default]
    Chip8,
    Chip8x,
    Chip48,
    SuperChip8,
    XoChip,
}

#[derive(Debug, Default)]
pub struct Chip8;

impl<P: Platform<GraphicsApi: SupportedGraphicsApiChip8Display>> MachineFactory<P> for Chip8 {
    fn construct(&self, machine: MachineBuilder<P>) -> MachineBuilder<P> {
        let (machine, cpu_address_space) = machine.insert_address_space(12);
        let (machine, timer) = machine.insert_default_component::<Chip8TimerConfig>("timer");
        let (machine, audio) = machine.insert_default_component::<Chip8AudioConfig>("audio");
        let (machine, display) = machine.insert_default_component::<Chip8DisplayConfig>("display");
        let (machine, _) = machine.insert_component(
            "cpu",
            Chip8ProcessorConfig {
                cpu_address_space,
                timer,
                audio,
                display,
                frequency: Ratio::from_integer(1000),
                force_mode: None,
                always_shr_in_place: false,
            },
        );
        let rom = machine.user_specified_roms().unwrap().main.clone();

        let (machine, _) = machine.insert_component(
            "workram",
            StandardMemoryConfig {
                readable: true,
                writable: true,
                assigned_range: 0x000..=0xfff,
                assigned_address_space: cpu_address_space,
                initial_contents: RangeInclusiveMap::from_iter([
                    (
                        0x000..=0x04f,
                        StandardMemoryInitialContents::Array(Cow::Borrowed(bytemuck::cast_slice(
                            &CHIP8_FONT,
                        ))),
                    ),
                    (0x200..=0xfff, StandardMemoryInitialContents::Rom(rom.id)),
                ]),
                sram: false,
            },
        );

        machine
    }
}
