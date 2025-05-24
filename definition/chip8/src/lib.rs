use audio::Chip8AudioConfig;
use display::{Chip8DisplayConfig, SupportedRenderApiChip8Display};
use font::CHIP8_FONT;
use multiemu_config::Environment;
use multiemu_definition_misc::memory::standard::{
    StandardMemoryConfig, StandardMemoryInitialContents,
};
use multiemu_machine::{MachineFactory, builder::MachineBuilder, display::shader::ShaderCache};
use multiemu_rom::{
    id::RomId,
    manager::RomManager,
    system::{GameSystem, OtherSystem},
};
use num::rational::Ratio;
use processor::Chip8ProcessorConfig;
pub use processor::decoder::Chip8InstructionDecoder;
use rangemap::RangeInclusiveMap;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    sync::{Arc, RwLock},
};
use timer::Chip8TimerConfig;

mod audio;
mod display;
mod font;
mod processor;
mod timer;

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug, Default)]
pub enum Chip8Kind {
    #[default]
    Chip8,
    Chip8x,
    Chip48,
    SuperChip8,
    XoChip,
}

trait SupportedRenderApiChip8: SupportedRenderApiChip8Display {}
impl<A: SupportedRenderApiChip8Display> SupportedRenderApiChip8 for A {}

#[derive(Debug, Default)]
pub struct Chip8;

impl<R: SupportedRenderApiChip8> MachineFactory<R> for Chip8 {
    fn construct(
        &self,
        user_specified_roms: Vec<RomId>,
        rom_manager: Arc<RomManager>,
        environment: Arc<RwLock<Environment>>,
        shader_cache: ShaderCache,
    ) -> MachineBuilder<R> {
        let machine = MachineBuilder::new(
            GameSystem::Other(OtherSystem::Chip8),
            rom_manager,
            environment,
            shader_cache,
        );

        let (machine, cpu_address_space) = machine.insert_address_space("cpu", 12);
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
                frequency: Ratio::from_integer(700),
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
