use bitvec::{prelude::Lsb0, view::BitView};
use multiemu_machine::{
    builder::ComponentBuilder,
    component::{Component, FromConfig, RuntimeEssentials},
    memory::{
        AddressSpaceId,
        callbacks::Memory,
        memory_translation_table::{ReadMemoryRecord, WriteMemoryRecord},
    },
};
use rangemap::RangeInclusiveMap;
use std::sync::Arc;
use strum::FromRepr;

use crate::CPU_ADDRESS_SPACE;

#[derive(Debug, PartialEq, Eq, Clone, Copy, FromRepr)]
enum WriteOnlyRegisters {
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
    Vdelpl = 0x027,
    Resmp0 = 0x028,
    Resmp1 = 0x029,
    Hmove = 0x02a,
    Hmclr = 0x02b,
    Cxclr = 0x02c,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, FromRepr)]
pub enum ReadOnlyRegisters {
    Cxm0p = 0x030,
    Cxm1p = 0x031,
    Cxp0fb = 0x032,
    Cxp1fb = 0x033,
    Cxm0fb = 0x034,
    Cxn1fb = 0x035,
    Cxblpf = 0x036,
    Cxppmm = 0x037,
    Inpt0 = 0x038,
    Inpt1 = 0x039,
    Inpt2 = 0x03a,
    Inpt3 = 0x03b,
    Inpt4 = 0x03c,
    Inpt5 = 0x03d,
}

pub struct Tia {}

impl Component for Tia {}

impl FromConfig for Tia {
    type Config = ();
    type Quirks = ();

    fn from_config(
        component_builder: ComponentBuilder<Self>,
        essentials: Arc<RuntimeEssentials>,
        config: Self::Config,
        quirks: Self::Quirks,
    ) {
        component_builder
            .insert_memory([(0x000..=0x03d, CPU_ADDRESS_SPACE)], MemoryCallback {})
            .build(Tia {});
    }
}

struct MemoryCallback {}

impl Memory for MemoryCallback {
    fn write_memory(
        &self,
        address: usize,
        _address_space: AddressSpaceId,
        buffer: &[u8],
        errors: &mut RangeInclusiveMap<usize, WriteMemoryRecord>,
    ) {
        let data = buffer[0].view_bits::<Lsb0>();

        if let Some(address) = WriteOnlyRegisters::from_repr(address) {
            match address {
                WriteOnlyRegisters::Vsync => {
                    let set_clear = data[1];
                }
                WriteOnlyRegisters::Vblank => {
                    let start_blanking = data[1];
                    let latch_inpt4to5 = data[6];
                    let dump_to_ground_inpt0to3 = data[7];
                }
                WriteOnlyRegisters::Wsync => todo!(),
                WriteOnlyRegisters::Rsync => todo!(),
                WriteOnlyRegisters::Nusiz0 => todo!(),
                WriteOnlyRegisters::Nusiz1 => todo!(),
                WriteOnlyRegisters::Colup0 => todo!(),
                WriteOnlyRegisters::Colup1 => todo!(),
                WriteOnlyRegisters::Colupf => todo!(),
                WriteOnlyRegisters::Colubk => todo!(),
                WriteOnlyRegisters::Ctrlpf => todo!(),
                WriteOnlyRegisters::Refp0 => todo!(),
                WriteOnlyRegisters::Refp1 => todo!(),
                WriteOnlyRegisters::Pf0 => todo!(),
                WriteOnlyRegisters::Pf1 => todo!(),
                WriteOnlyRegisters::Pf2 => todo!(),
                WriteOnlyRegisters::Resp0 => todo!(),
                WriteOnlyRegisters::Resp1 => todo!(),
                WriteOnlyRegisters::Resm0 => todo!(),
                WriteOnlyRegisters::Resm1 => todo!(),
                WriteOnlyRegisters::Resbl => todo!(),
                WriteOnlyRegisters::Audc0 => todo!(),
                WriteOnlyRegisters::Audc1 => todo!(),
                WriteOnlyRegisters::Audf0 => todo!(),
                WriteOnlyRegisters::Audf1 => todo!(),
                WriteOnlyRegisters::Audv0 => todo!(),
                WriteOnlyRegisters::Audv1 => todo!(),
                WriteOnlyRegisters::Grp0 => todo!(),
                WriteOnlyRegisters::Grp1 => todo!(),
                WriteOnlyRegisters::Enam0 => todo!(),
                WriteOnlyRegisters::Enam1 => todo!(),
                WriteOnlyRegisters::Enabl => todo!(),
                WriteOnlyRegisters::Hmp0 => todo!(),
                WriteOnlyRegisters::Hmp1 => todo!(),
                WriteOnlyRegisters::Hmm0 => todo!(),
                WriteOnlyRegisters::Hmm1 => todo!(),
                WriteOnlyRegisters::Hmbl => todo!(),
                WriteOnlyRegisters::Vdelp0 => todo!(),
                WriteOnlyRegisters::Vdelp1 => todo!(),
                WriteOnlyRegisters::Vdelpl => todo!(),
                WriteOnlyRegisters::Resmp0 => todo!(),
                WriteOnlyRegisters::Resmp1 => todo!(),
                WriteOnlyRegisters::Hmove => todo!(),
                WriteOnlyRegisters::Hmclr => todo!(),
                WriteOnlyRegisters::Cxclr => todo!(),
            }
        } else {
            errors.insert(
                address..=(address + (buffer.len() - 1)),
                WriteMemoryRecord::Denied,
            );
        }
    }

    fn read_memory(
        &self,
        address: usize,
        address_space: AddressSpaceId,
        buffer: &mut [u8],
        errors: &mut RangeInclusiveMap<usize, ReadMemoryRecord>,
    ) {
        if let Some(address) = ReadOnlyRegisters::from_repr(address) {
            match address {
                ReadOnlyRegisters::Cxm0p => todo!(),
                ReadOnlyRegisters::Cxm1p => todo!(),
                ReadOnlyRegisters::Cxp0fb => todo!(),
                ReadOnlyRegisters::Cxp1fb => todo!(),
                ReadOnlyRegisters::Cxm0fb => todo!(),
                ReadOnlyRegisters::Cxn1fb => todo!(),
                ReadOnlyRegisters::Cxblpf => todo!(),
                ReadOnlyRegisters::Cxppmm => todo!(),
                ReadOnlyRegisters::Inpt0 => todo!(),
                ReadOnlyRegisters::Inpt1 => todo!(),
                ReadOnlyRegisters::Inpt2 => todo!(),
                ReadOnlyRegisters::Inpt3 => todo!(),
                ReadOnlyRegisters::Inpt4 => todo!(),
                ReadOnlyRegisters::Inpt5 => todo!(),
            }
        } else {
            errors.insert(
                address..=(address + (buffer.len() - 1)),
                ReadMemoryRecord::Denied,
            );
        }
    }
}
