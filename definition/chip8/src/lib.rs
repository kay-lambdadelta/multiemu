use audio::Chip8Audio;
use display::Chip8Display;
use font::CHIP8_FONT;
use multiemu_config::Environment;
use multiemu_definition_misc::memory::standard::{
    StandardMemory, StandardMemoryConfig, StandardMemoryInitialContents,
};
use multiemu_machine::{builder::MachineBuilder, display::shader::ShaderCache};
use multiemu_rom::{
    id::RomId,
    manager::RomManager,
    system::{GameSystem, OtherSystem},
};
pub use processor::decoder::Chip8InstructionDecoder;
use processor::{Chip8Processor, Chip8ProcessorConfig};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    sync::{Arc, RwLock},
};
use timer::Chip8Timer;

mod audio;
mod display;
mod font;
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

/// Create a new chip8 machine
pub fn manifest(
    user_specified_roms: Vec<RomId>,
    rom_manager: Arc<RomManager>,
    environment: Arc<RwLock<Environment>>,
    shader_cache: ShaderCache,
) -> MachineBuilder {
    let machine = MachineBuilder::new(
        GameSystem::Other(OtherSystem::Chip8),
        rom_manager,
        environment,
        shader_cache,
    );

    let (machine, cpu_address_space) = machine.insert_address_space("cpu", 12);

    let (machine, _) = machine.insert_default_component::<Chip8Timer>("timer");
    let (machine, _) = machine.insert_default_component::<Chip8Audio>("audio");
    let (machine, _) = machine.insert_default_component::<Chip8Display>("display");
    let (machine, _) = machine
        .insert_component::<Chip8Processor>("cpu", Chip8ProcessorConfig { cpu_address_space });
    let (machine, _) = machine.insert_component::<StandardMemory>(
        "workram",
        StandardMemoryConfig {
            readable: true,
            writable: true,
            max_word_size: 2,
            assigned_range: 0x000..=0xfff,
            assigned_address_space: cpu_address_space,
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
