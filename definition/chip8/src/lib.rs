use std::{borrow::Cow, marker::PhantomData};

use audio::Chip8AudioConfig;
use display::Chip8DisplayConfig;
use font::CHIP8_FONT;
use multiemu_definition_misc::memory::standard::{
    StandardMemoryConfig, StandardMemoryInitialContents,
};
use multiemu_runtime::{
    machine::{MachineFactory, builder::MachineBuilder},
    platform::Platform,
    program::Filesystem,
    scheduler::Frequency,
};
use processor::Chip8ProcessorConfig;
use rangemap::RangeInclusiveMap;
use serde::{Deserialize, Serialize};
use timer::Chip8TimerConfig;

use crate::display::SupportedGraphicsApiChip8Display;

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
        let (machine, audio) = machine.insert_component(
            "audio",
            Chip8AudioConfig {
                processor_frequency: Frequency::from_num(1000),
            },
        );
        let (machine, display) = machine.insert_default_component::<Chip8DisplayConfig>("display");
        let (machine, _) = machine.insert_component(
            "cpu",
            Chip8ProcessorConfig {
                cpu_address_space,
                timer,
                audio,
                display,
                frequency: Frequency::from_num(1000),
                force_mode: None,
                always_shr_in_place: false,
                _phantom: PhantomData,
            },
        );

        let Filesystem::Single { rom_id, .. } =
            machine.program_specification().unwrap().info.filesystem()
        else {
            panic!("No atari 2600 game has a structured filesystem")
        };
        let rom = *rom_id;

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
                    (0x200..=0xfff, StandardMemoryInitialContents::Rom(rom)),
                ]),
                sram: false,
            },
        );

        machine
    }
}
