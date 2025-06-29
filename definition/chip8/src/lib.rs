use audio::Chip8AudioConfig;
use display::Chip8DisplayConfig;
use font::CHIP8_FONT;
use multiemu_definition_misc::memory::standard::{
    StandardMemoryConfig, StandardMemoryInitialContents,
};
use multiemu_rom::{RomId, RomManager};
use multiemu_runtime::{MachineFactory, builder::MachineBuilder, platform::Platform};
use num::rational::Ratio;
use processor::Chip8ProcessorConfig;
pub use processor::decoder::Chip8InstructionDecoder;
use rangemap::RangeInclusiveMap;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, sync::Arc};
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
    fn construct(
        &self,
        user_specified_roms: Vec<RomId>,
        rom_manager: Arc<RomManager>,
        sample_rate: Ratio<u32>,
        main_thread_executor: Arc<P::MainThreadExecutor>,
    ) -> MachineBuilder<P> {
        let machine = MachineBuilder::new(rom_manager, sample_rate, main_thread_executor);

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
        let (machine, _) = machine.insert_component(
            "workram",
            StandardMemoryConfig {
                readable: true,
                writable: true,
                assigned_range: 0x000..=0xfff,
                assigned_address_space: cpu_address_space,
                initial_contents: RangeInclusiveMap::from_iter([
                    (
                        0x000..=0x1ff,
                        StandardMemoryInitialContents::Array(Cow::Borrowed(bytemuck::cast_slice(
                            &CHIP8_FONT,
                        ))),
                    ),
                    (
                        0x200..=0xfff,
                        StandardMemoryInitialContents::Rom(user_specified_roms[0]),
                    ),
                ]),
            },
        );

        machine
    }
}
