use super::decoder::InstructionDecoder;
use crate::memory::memory_translation_table::MemoryTranslationTable;
use rangemap::RangeInclusiveMap;
use std::sync::Mutex;

#[derive(Debug)]
pub struct InstructionCache<ID: InstructionDecoder> {
    decoder: ID,
    decode_cache: Mutex<RangeInclusiveMap<usize, ID::InstructionSet>>,
}

impl<ID: InstructionDecoder> Default for InstructionCache<ID>
where
    ID: Default,
{
    fn default() -> Self {
        Self {
            decoder: ID::default(),
            decode_cache: Mutex::new(RangeInclusiveMap::default()),
        }
    }
}

impl<ID: InstructionDecoder> InstructionCache<ID> {
    pub fn new(decoder: ID) -> Self {
        Self {
            decoder,
            decode_cache: Mutex::new(RangeInclusiveMap::default()),
        }
    }
}

impl<ID: InstructionDecoder> InstructionDecoder for InstructionCache<ID> {
    type InstructionSet = ID::InstructionSet;

    fn decode(
        &self,
        cursor: usize,
        memory_translation_table: &MemoryTranslationTable,
    ) -> Option<(Self::InstructionSet, u8)> {
        // TODO: Implement cache eviction
        let mut decode_cache_guard = self.decode_cache.lock().unwrap();
        let mut should_remove = None;

        if let Some((occupying_range, instruction)) = decode_cache_guard.get_key_value(&cursor) {
            // Unaligned instruction
            if *occupying_range.start() != cursor {
                should_remove = Some(occupying_range.clone());
            } else {
                return Some((
                    instruction.clone(),
                    occupying_range.clone().count().try_into().unwrap(),
                ));
            }
        }

        if let Some(should_remove) = should_remove {
            decode_cache_guard.remove(should_remove);
        }

        if let Some((instruction, instruction_length)) =
            self.decoder.decode(cursor, memory_translation_table)
        {
            decode_cache_guard.insert(
                cursor..=(cursor + instruction_length as usize - 1),
                instruction.clone(),
            );

            Some((instruction, instruction_length))
        } else {
            None
        }
    }
}
