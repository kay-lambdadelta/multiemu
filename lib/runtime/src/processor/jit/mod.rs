use super::instruction::InstructionSet;
use crate::memory::memory_access_table::MemoryAccessTable;
use cranelift::{
    jit::{JITBuilder, JITModule},
    module::default_libcall_names,
    prelude::{Block, Configurable, FunctionBuilder, Value, settings::Flags},
};
use processor_state::ProcessorState;
use rangemap::RangeInclusiveMap;

pub mod processor_state;

pub const MAX_INSTRUCTION_FUSION: usize = 32;

pub trait InstructionTranslator: Send + Sync {
    type InstructionSet: InstructionSet;

    fn translate(
        &self,
        processor_state: Value,
        function_builder: &mut FunctionBuilder,
        instructions: RangeInclusiveMap<usize, Self::InstructionSet>,
        blocks: &mut RangeInclusiveMap<usize, Block>,
    );
}

pub struct InstructionJitExecutor<T: InstructionTranslator>
where
    Self: Send,
{
    translator: T,
    blocks: RangeInclusiveMap<usize, Block>,
    module: JITModule,
}

impl<T: InstructionTranslator> InstructionJitExecutor<T> {
    pub fn new(translator: T) -> Self {
        let mut flag_builder = cranelift::prelude::settings::builder();
        flag_builder.set("use_colocated_libcalls", "false").unwrap();
        flag_builder.set("is_pic", "true").unwrap();
        let isa_builder = cranelift::native::builder().unwrap_or_else(|msg| {
            unimplemented!("Cannot perform jit recompilation on your machine: {}", msg);
        });
        let isa = isa_builder.finish(Flags::new(flag_builder)).unwrap();

        tracing::info!("Initializing jit executor on a {} machine", isa.triple());

        let mut builder = JITBuilder::with_isa(isa, default_libcall_names());
        builder.hotswap(true);

        let module = JITModule::new(builder);

        Self {
            translator,
            blocks: RangeInclusiveMap::default(),
            module,
        }
    }
}

impl<T: InstructionTranslator> InstructionJitExecutor<T> {
    pub fn run<PS: ProcessorState>(
        &mut self,
        cursor: usize,
        memory_access_table: &MemoryAccessTable,
        state: &mut PS,
    ) {
    }
}
