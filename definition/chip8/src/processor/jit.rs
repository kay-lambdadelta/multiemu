use super::instruction::{Chip8InstructionSet, InstructionSetChip8, Register};
use crate::Chip8Kind;
use crate::processor::ProcessorState;
use cranelift::prelude::{Block, FunctionBuilder, InstBuilder, MemFlags, Value, types};
use rangemap::RangeInclusiveMap;
use std::{mem::offset_of, sync::Arc};

impl ProcessorState {
    #[inline]
    pub fn get_work_register_offset(register: Register) -> u8 {
        (offset_of!(ProcessorState, registers.work_registers) + register as usize * 2) as u8
    }
}

#[derive(Debug)]
pub struct Chip8InstructionTranslator {
    pub mode: Arc<AtomicCell<Chip8Kind>>,
}

impl Chip8InstructionTranslator {
    pub fn new(mode: Arc<AtomicCell<Chip8Kind>>) -> Chip8InstructionTranslator {
        Chip8InstructionTranslator { mode }
    }
}

impl InstructionTranslator for Chip8InstructionTranslator {
    type InstructionSet = Chip8InstructionSet;

    fn translate(
        &self,
        processor_state: Value,
        function_builder: &mut FunctionBuilder,
        instructions: RangeInclusiveMap<usize, Self::InstructionSet>,
        blocks: &mut RangeInclusiveMap<usize, Block>,
    ) {
        for (cursor, instruction) in instructions {
            let current_block = function_builder.create_block();
            blocks.insert(cursor, current_block);

            match instruction {
                Chip8InstructionSet::Chip8(InstructionSetChip8::Add {
                    register,
                    immediate,
                }) => {
                    let register_location = ProcessorState::get_work_register_offset(register);

                    let register_value = function_builder.ins().load(
                        types::I16,
                        MemFlags::new(),
                        processor_state,
                        register_location,
                    );

                    let immediate_value =
                        function_builder.ins().iconst(types::I8, immediate as i64);

                    let new_value = function_builder.ins().iadd(register_value, immediate_value);

                    function_builder.ins().store(
                        MemFlags::new(),
                        new_value,
                        processor_state,
                        register_location,
                    );
                }
                Chip8InstructionSet::Chip8(InstructionSetChip8::Sys { .. }) => {}
                Chip8InstructionSet::Chip8(InstructionSetChip8::Jump { .. }) => {}
                Chip8InstructionSet::Chip8(InstructionSetChip8::Call { .. }) => {}
                Chip8InstructionSet::Chip8(InstructionSetChip8::Ske { .. }) => {}
                Chip8InstructionSet::Chip8(InstructionSetChip8::Skne { .. }) => {}
                Chip8InstructionSet::Chip8(InstructionSetChip8::Skre { .. }) => {}
                Chip8InstructionSet::Chip8(InstructionSetChip8::Load { .. }) => {}
                Chip8InstructionSet::Chip8(InstructionSetChip8::Move { .. }) => {}
                Chip8InstructionSet::Chip8(InstructionSetChip8::Or {
                    destination,
                    source,
                }) => {
                    let destination_location =
                        ProcessorState::get_work_register_offset(destination);
                    let destination_value = function_builder.ins().load(
                        types::I16,
                        MemFlags::new(),
                        processor_state,
                        destination_location,
                    );

                    let source_location = ProcessorState::get_work_register_offset(source);
                    let source_value = function_builder.ins().load(
                        types::I16,
                        MemFlags::new(),
                        processor_state,
                        source_location,
                    );

                    let new_value = function_builder.ins().bor(destination_value, source_value);

                    function_builder.ins().store(
                        MemFlags::new(),
                        new_value,
                        processor_state,
                        destination_location,
                    );

                    if self.mode.load() == Chip8Kind::Chip8 {
                        let zero = function_builder.ins().iconst(types::I16, 0);

                        function_builder.ins().store(
                            MemFlags::new(),
                            zero,
                            processor_state,
                            ProcessorState::get_work_register_offset(Register::V0),
                        );
                    }
                }
                Chip8InstructionSet::Chip8(InstructionSetChip8::And {
                    destination,
                    source,
                }) => {
                    let destination_location =
                        ProcessorState::get_work_register_offset(destination);
                    let destination_value = function_builder.ins().load(
                        types::I16,
                        MemFlags::new(),
                        processor_state,
                        destination_location,
                    );

                    let source_location = ProcessorState::get_work_register_offset(source);
                    let source_value = function_builder.ins().load(
                        types::I16,
                        MemFlags::new(),
                        processor_state,
                        source_location,
                    );

                    let new_value = function_builder.ins().band(destination_value, source_value);

                    function_builder.ins().store(
                        MemFlags::new(),
                        new_value,
                        processor_state,
                        destination_location,
                    );

                    if self.mode.load() == Chip8Kind::Chip8 {
                        let zero = function_builder.ins().iconst(types::I16, 0);

                        function_builder.ins().store(
                            MemFlags::new(),
                            zero,
                            processor_state,
                            ProcessorState::get_work_register_offset(Register::V0),
                        );
                    }
                }
                Chip8InstructionSet::Chip8(InstructionSetChip8::Xor {
                    destination,
                    source,
                }) => {
                    let destination_location =
                        ProcessorState::get_work_register_offset(destination);
                    let destination_value = function_builder.ins().load(
                        types::I16,
                        MemFlags::new(),
                        processor_state,
                        destination_location,
                    );

                    let source_location = ProcessorState::get_work_register_offset(source);
                    let source_value = function_builder.ins().load(
                        types::I16,
                        MemFlags::new(),
                        processor_state,
                        source_location,
                    );

                    let new_value = function_builder.ins().bxor(destination_value, source_value);

                    function_builder.ins().store(
                        MemFlags::new(),
                        new_value,
                        processor_state,
                        destination_location,
                    );

                    if self.mode.load() == Chip8Kind::Chip8 {
                        let zero = function_builder.ins().iconst(types::I16, 0);

                        function_builder.ins().store(
                            MemFlags::new(),
                            zero,
                            processor_state,
                            ProcessorState::get_work_register_offset(Register::V0),
                        );
                    }
                }
                Chip8InstructionSet::Chip8(InstructionSetChip8::Addr { .. }) => {}
                Chip8InstructionSet::Chip8(InstructionSetChip8::Sub { .. }) => {}
                Chip8InstructionSet::Chip8(InstructionSetChip8::Shr { .. }) => {}
                Chip8InstructionSet::Chip8(InstructionSetChip8::Subn { .. }) => {}
                Chip8InstructionSet::Chip8(InstructionSetChip8::Shl { .. }) => {}
                Chip8InstructionSet::Chip8(InstructionSetChip8::Skrne { .. }) => {}
                Chip8InstructionSet::Chip8(InstructionSetChip8::Loadi { .. }) => {}
                Chip8InstructionSet::Chip8(InstructionSetChip8::Jumpi { .. }) => {}
                Chip8InstructionSet::Chip8(InstructionSetChip8::Rand { .. }) => {}
                Chip8InstructionSet::Chip8(InstructionSetChip8::Draw { .. }) => {}
                Chip8InstructionSet::Chip8(InstructionSetChip8::Skpr { .. }) => {}
                Chip8InstructionSet::Chip8(InstructionSetChip8::Skup { .. }) => {}
                Chip8InstructionSet::Chip8(InstructionSetChip8::Moved { .. }) => {}
                Chip8InstructionSet::Chip8(InstructionSetChip8::Keyd { .. }) => {}
                Chip8InstructionSet::Chip8(InstructionSetChip8::Loadd { .. }) => {}
                Chip8InstructionSet::Chip8(InstructionSetChip8::Loads { .. }) => {}
                Chip8InstructionSet::Chip8(InstructionSetChip8::Addi { .. }) => {}
                Chip8InstructionSet::Chip8(InstructionSetChip8::Font { .. }) => {}
                Chip8InstructionSet::Chip8(InstructionSetChip8::Bcd { .. }) => {}
                Chip8InstructionSet::Chip8(InstructionSetChip8::Save { .. }) => {}
                Chip8InstructionSet::Chip8(InstructionSetChip8::Restore { .. }) => {}
                Chip8InstructionSet::SuperChip8(_) => {}
                Chip8InstructionSet::XoChip(_) => {}
            }
        }
    }
}
