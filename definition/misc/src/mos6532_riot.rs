use crate::memory::standard::{StandardMemoryConfig, StandardMemoryInitialContents};
use multiemu_range::ContiguousRange;
use multiemu_runtime::{
    component::{Component, ComponentConfig, ComponentVersion},
    machine::builder::ComponentBuilder,
    memory::{
        Address, AddressSpaceId, ReadMemoryError, ReadMemoryErrorType, WriteMemoryError,
        WriteMemoryErrorType,
    },
    platform::Platform,
    scheduler::TaskType,
};
use num::rational::Ratio;
use rangemap::RangeInclusiveMap;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::{
    fmt::Debug,
    io::{Read, Write},
    num::NonZero,
    ops::RangeInclusive,
    sync::OnceLock,
};

#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct Snapshot {
    swacnt: bool,
    swbcnt: bool,
    intim: u8,
    instat: u8,
    tim1t: u8,
    tim8t: u8,
    tim64t: u8,
    t1024t: u8,
}

#[derive(Debug)]
struct Registers {
    swcha: OnceLock<Box<dyn SwchaCallback>>,
    swchb: OnceLock<Box<dyn SwchbCallback>>,
    swacnt: bool,
    swbcnt: bool,
    intim: u8,
    instat: u8,
    tim1t: u8,
    tim8t: u8,
    tim64t: u8,
    t1024t: u8,
}

pub trait SwchaCallback: Debug + Send + Sync + 'static {
    fn read_register(&self) -> u8;
    fn write_register(&self, value: u8);
}

pub trait SwchbCallback: Debug + Send + Sync + 'static {
    fn read_register(&self) -> u8;
    fn write_register(&self, value: u8);
}

#[derive(Debug)]
pub struct Mos6532Riot {
    registers: Registers,
    config: Mos6532RiotConfig,
}

impl Mos6532Riot {
    pub fn install_swcha(&self, callback: impl SwchaCallback) {
        self.registers
            .swcha
            .set(Box::new(callback))
            .expect("SWCHA already set");
    }

    pub fn install_swchb(&self, callback: impl SwchbCallback) {
        self.registers
            .swchb
            .set(Box::new(callback))
            .expect("SWCHA already set");
    }
}

impl Component for Mos6532Riot {
    fn snapshot_version(&self) -> Option<ComponentVersion> {
        Some(0)
    }

    fn store_snapshot(&self, mut writer: Box<dyn Write>) -> Result<(), Box<dyn std::error::Error>> {
        let snapshot = Snapshot {
            swacnt: self.registers.swacnt,
            swbcnt: self.registers.swbcnt,
            intim: self.registers.intim,
            instat: self.registers.instat,
            tim1t: self.registers.tim1t,
            tim8t: self.registers.tim8t,
            tim64t: self.registers.tim64t,
            t1024t: self.registers.t1024t,
        };

        bincode::serde::encode_into_std_write(&snapshot, &mut writer, bincode::config::standard())?;

        Ok(())
    }

    fn load_snapshot(
        &mut self,
        version: ComponentVersion,
        mut reader: Box<dyn Read>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match version {
            0 => {
                // Decode the snapshot from the file
                let snapshot: Snapshot =
                    bincode::serde::decode_from_std_read(&mut reader, bincode::config::standard())?;

                // Restore state into atomics
                self.registers.swacnt = snapshot.swacnt;
                self.registers.swbcnt = snapshot.swbcnt;
                self.registers.intim = snapshot.intim;
                self.registers.instat = snapshot.instat;
                self.registers.tim1t = snapshot.tim1t;
                self.registers.tim8t = snapshot.tim8t;
                self.registers.tim64t = snapshot.tim64t;
                self.registers.t1024t = snapshot.t1024t;

                Ok(())
            }
            _ => Err(format!("Unsupported snapshot version: {version}").into()),
        }
    }

    fn read_memory(
        &self,
        address: Address,
        _address_space: AddressSpaceId,
        avoid_side_effects: bool,
        buffer: &mut [u8],
    ) -> Result<(), ReadMemoryError> {
        for (address, buffer_section) in
            RangeInclusive::from_start_and_length(address, buffer.len()).zip(buffer.iter_mut())
        {
            let adjusted_address = address - self.config.registers_assigned_address;

            match adjusted_address {
                0x0 | 0x2 if avoid_side_effects => {
                    return Err(ReadMemoryError(
                        std::iter::once((
                            address..=(address + (buffer.len() - 1)),
                            ReadMemoryErrorType::Denied,
                        ))
                        .collect(),
                    ));
                }
                0x0 if self.registers.swacnt => {
                    *buffer_section = self
                        .registers
                        .swcha
                        .get()
                        .map_or(0, |handler| handler.read_register());
                }
                0x1 => {
                    *buffer_section = u8::from(self.registers.swacnt);
                }
                0x2 if self.registers.swbcnt => {
                    *buffer_section = self
                        .registers
                        .swchb
                        .get()
                        .map_or(0, |handler| handler.read_register());
                }
                0x3 => {
                    *buffer_section = u8::from(self.registers.swbcnt);
                }
                0x4 => {
                    *buffer_section = self.registers.intim;
                }
                0x5 => {
                    *buffer_section = self.registers.instat;
                }
                0x14 => {
                    *buffer_section = self.registers.tim1t;
                }
                0x15 => {
                    *buffer_section = self.registers.tim8t;
                }
                0x16 => {
                    *buffer_section = self.registers.tim64t;
                }
                0x17 => {
                    *buffer_section = self.registers.t1024t;
                }
                _ => {
                    return Err(ReadMemoryError(
                        std::iter::once((
                            address..=(address + (buffer.len() - 1)),
                            ReadMemoryErrorType::Denied,
                        ))
                        .collect(),
                    ));
                }
            }
        }

        Ok(())
    }

    fn write_memory(
        &mut self,
        address: Address,
        _address_space: AddressSpaceId,
        buffer: &[u8],
    ) -> Result<(), WriteMemoryError> {
        for (address, buffer_section) in
            RangeInclusive::from_start_and_length(address, buffer.len()).zip(buffer.iter())
        {
            let adjusted_address = address - self.config.registers_assigned_address;

            match adjusted_address {
                0x0 if !self.registers.swacnt => {
                    self.registers
                        .swcha
                        .get()
                        .unwrap()
                        .write_register(*buffer_section);
                }
                0x1 => {
                    self.registers.swacnt = *buffer_section != 0;
                }
                0x2 if !self.registers.swbcnt => {
                    self.registers
                        .swchb
                        .get()
                        .unwrap()
                        .write_register(*buffer_section);
                }
                0x3 => {
                    self.registers.swbcnt = *buffer_section != 0;
                }
                0x14 => {
                    self.registers.tim1t = *buffer_section;
                }
                0x15 => {
                    self.registers.tim8t = *buffer_section;
                }
                0x16 => {
                    self.registers.tim64t = *buffer_section;
                }
                0x17 => {
                    self.registers.t1024t = *buffer_section;
                }
                _ => {
                    return Err(WriteMemoryError(
                        std::iter::once((
                            address..=(address + (buffer.len() - 1)),
                            WriteMemoryErrorType::Denied,
                        ))
                        .collect(),
                    ));
                }
            }
        }

        Ok(())
    }
}

impl<P: Platform> ComponentConfig<P> for Mos6532RiotConfig {
    type Component = Mos6532Riot;

    fn build_component(
        self,
        component_builder: ComponentBuilder<'_, P, Self::Component>,
    ) -> Result<Self::Component, Box<dyn std::error::Error>> {
        let registers = Registers {
            swcha: OnceLock::new(),
            swchb: OnceLock::new(),
            swacnt: false,
            swbcnt: false,
            intim: 0,
            instat: 0,
            tim1t: 0,
            tim8t: 0,
            tim64t: 0,
            t1024t: 0,
        };

        let ram_assigned_addresses =
            self.ram_assigned_address..=self.ram_assigned_address.checked_add(0x7f).unwrap();

        let component_builder = component_builder.memory_map(
            self.registers_assigned_address..=self.registers_assigned_address + 0x1f,
            self.assigned_address_space,
        );

        let (component_builder, _) = component_builder.insert_child_component(
            "ram",
            StandardMemoryConfig {
                readable: true,
                writable: true,
                assigned_range: ram_assigned_addresses.clone(),
                assigned_address_space: self.assigned_address_space,
                initial_contents: RangeInclusiveMap::from_iter([(
                    ram_assigned_addresses,
                    StandardMemoryInitialContents::Random,
                )]),
                sram: false,
            },
        );

        set_up_timer_tasks(&self, component_builder);

        Ok(Self::Component {
            registers,
            config: self,
        })
    }
}

fn set_up_timer_tasks<'a, P: Platform>(
    config: &Mos6532RiotConfig,
    component_builder: ComponentBuilder<'a, P, Mos6532Riot>,
) -> ComponentBuilder<'a, P, Mos6532Riot> {
    // Make the timers operate
    component_builder
        .insert_task(
            "tim1t",
            config.frequency,
            TaskType::Lazy,
            move |component: &mut Mos6532Riot, slice: NonZero<u32>| {
                component.registers.tim1t = component
                    .registers
                    .tim1t
                    .wrapping_add(slice.get().try_into().unwrap_or(u8::MAX));
            },
        )
        .0
        .insert_task(
            "tim8t",
            config.frequency / 8,
            TaskType::Lazy,
            move |component: &mut Mos6532Riot, slice: NonZero<u32>| {
                component.registers.tim8t = component
                    .registers
                    .tim8t
                    .wrapping_add(slice.get().try_into().unwrap_or(u8::MAX));
            },
        )
        .0
        .insert_task(
            "tim64t",
            config.frequency / 64,
            TaskType::Lazy,
            move |component: &mut Mos6532Riot, slice: NonZero<u32>| {
                component.registers.tim64t = component
                    .registers
                    .tim64t
                    .wrapping_add(slice.get().try_into().unwrap_or(u8::MAX));
            },
        )
        .0
        .insert_task(
            "t1024t",
            config.frequency / 1024,
            TaskType::Lazy,
            move |component: &mut Mos6532Riot, slice: NonZero<u32>| {
                component.registers.t1024t = component
                    .registers
                    .t1024t
                    .wrapping_add(slice.get().try_into().unwrap_or(u8::MAX));
            },
        )
        .0
}

#[derive(Debug)]
pub struct Mos6532RiotConfig {
    pub frequency: Ratio<u32>,
    pub registers_assigned_address: Address,
    pub ram_assigned_address: Address,
    pub assigned_address_space: AddressSpaceId,
}
