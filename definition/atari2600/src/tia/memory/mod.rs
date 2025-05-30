use super::State;
use bitvec::{order::Lsb0, view::BitView};
use multiemu_definition_mos6502::Mos6502;
use multiemu_machine::{
    component::component_ref::ComponentRef,
    memory::{
        Address,
        callbacks::{Memory, ReadMemory, WriteMemory},
        memory_translation_table::{
            MemoryOperationError, ReadMemoryRecord, WriteMemoryRecord,
            address_space::AddressSpaceHandle,
        },
    },
};
use rangemap::RangeInclusiveMap;
use std::sync::{Arc, Mutex};
use strum::FromRepr;

mod read;
mod write;

#[derive(Debug, PartialEq, Eq, Clone, Copy, FromRepr)]
enum ReadRegisters {
    Cxm0p = 0x000,
    Cxm1p = 0x001,
    Cxp0fb = 0x002,
    Cxp1fb = 0x003,
    Cxm0fb = 0x004,
    Cxm1fb = 0x005,
    Cxblpf = 0x006,
    Cxppmm = 0x007,
    Inpt0 = 0x008,
    Inpt1 = 0x009,
    Inpt2 = 0x00a,
    Inpt3 = 0x00b,
    Inpt4 = 0x00c,
    Inpt5 = 0x00d,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, FromRepr)]
enum WriteRegisters {
    Vsync = 0x000,
    Vblank = 0x001,
    Wsync = 0x002,
    Rsync = 0x003,
    Nusiz0 = 0x004,
    Nusiz1 = 0x005,
    Colup0 = 0x006,
    Colup1 = 0x007,
    Colupf = 0x008,
    Colubk = 0x009,
    Ctrlpf = 0x00a,
    Refp0 = 0x00b,
    Refp1 = 0x00c,
    Pf0 = 0x00d,
    Pf1 = 0x00e,
    Pf2 = 0x00f,
    Resp0 = 0x010,
    Resp1 = 0x011,
    Resm0 = 0x012,
    Resm1 = 0x013,
    Resbl = 0x014,
    Audc0 = 0x015,
    Audc1 = 0x016,
    Audf0 = 0x017,
    Audf1 = 0x018,
    Audv0 = 0x019,
    Audv1 = 0x01a,
    Grp0 = 0x01b,
    Grp1 = 0x01c,
    Enam0 = 0x01d,
    Enam1 = 0x01e,
    Enabl = 0x01f,
    Hmp0 = 0x020,
    Hmp1 = 0x021,
    Hmm0 = 0x022,
    Hmm1 = 0x023,
    Hmbl = 0x024,
    Vdelp0 = 0x025,
    Vdelp1 = 0x026,
    Vdelbl = 0x027,
    Resmp0 = 0x028,
    Resmp1 = 0x029,
    Hmove = 0x02a,
    Hmclr = 0x02b,
    Cxclr = 0x02c,
}

#[derive(Debug)]
pub struct MemoryCallback {
    pub state: Arc<Mutex<State>>,
    pub cpu: ComponentRef<Mos6502>,
}

impl Memory for MemoryCallback {}

impl WriteMemory for MemoryCallback {
    fn write_memory(
        &self,
        address: Address,
        _address_space: AddressSpaceHandle,
        buffer: &[u8],
    ) -> Result<(), MemoryOperationError<WriteMemoryRecord>> {
        let data = buffer[0];
        let data_bits = data.view_bits::<Lsb0>();
        let mut state_guard = self.state.lock().unwrap();

        if let Some(address) = WriteRegisters::from_repr(address) {
            tracing::debug!("Writing to TIA register: {:?} = {:02x}", address, data);

            self.handle_write_register(data, data_bits, &mut state_guard, address);

            Ok(())
        } else {
            Err(RangeInclusiveMap::from_iter([(
                address..=(address + (buffer.len() - 1)),
                WriteMemoryRecord::Denied,
            )])
            .into())
        }
    }
}

impl ReadMemory for MemoryCallback {
    fn read_memory(
        &self,
        address: Address,
        _address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), MemoryOperationError<ReadMemoryRecord>> {
        let data = &mut buffer[0];
        let mut state_guard = self.state.lock().unwrap();

        if let Some(address) = ReadRegisters::from_repr(address) {
            tracing::debug!("Reading from TIA register: {:?}", address);

            self.handle_read_register(data, &mut state_guard, address);

            Ok(())
        } else {
            Err(RangeInclusiveMap::from_iter([(
                address..=(address + (buffer.len() - 1)),
                ReadMemoryRecord::Denied,
            )])
            .into())
        }
    }
}
