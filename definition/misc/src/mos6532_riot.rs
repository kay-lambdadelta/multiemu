use std::{
    fmt::Debug,
    io::{Read, Write},
    num::Wrapping,
    ops::RangeInclusive,
};

use multiemu_range::ContiguousRange;
use multiemu_runtime::{
    component::{Component, ComponentConfig, ComponentVersion, SynchronizationContext},
    machine::builder::{ComponentBuilder, SchedulerParticipation},
    memory::{Address, AddressSpaceId, MemoryError, MemoryErrorType},
    platform::Platform,
    scheduler::{Frequency, Period},
};
use rangemap::RangeInclusiveMap;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::memory::standard::{StandardMemoryConfig, StandardMemoryInitialContents};

#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct Snapshot {
    swacnt: bool,
    swbcnt: bool,
    intim: u8,
    instat: u8,
    tim1t: Wrapping<u8>,
    tim8t: Counter,
    tim64t: Counter,
    t1024t: Counter,
}

pub trait SwchaCallback: Debug + Send + Sync + 'static {
    fn read_register(&self) -> u8;
    fn write_register(&self, value: u8);
}

pub trait SwchbCallback: Debug + Send + Sync + 'static {
    fn read_register(&self) -> u8;
    fn write_register(&self, value: u8);
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy)]
struct Counter {
    pub last_updated: u64,
    pub value: Wrapping<u8>,
}

#[derive(Debug)]
pub struct Mos6532Riot {
    swcha: Option<Box<dyn SwchaCallback>>,
    swchb: Option<Box<dyn SwchbCallback>>,
    swacnt: bool,
    swbcnt: bool,
    intim: u8,
    instat: u8,
    tim1t: Wrapping<u8>,
    tim8t: Counter,
    tim64t: Counter,
    t1024t: Counter,

    time_counter: u64,
    config: Mos6532RiotConfig,
}

impl Mos6532Riot {
    pub fn install_swcha(&mut self, callback: impl SwchaCallback) {
        self.swcha = Some(Box::new(callback));
    }

    pub fn install_swchb(&mut self, callback: impl SwchbCallback) {
        self.swchb = Some(Box::new(callback));
    }
}

impl Component for Mos6532Riot {
    fn snapshot_version(&self) -> Option<ComponentVersion> {
        Some(0)
    }

    fn store_snapshot(&self, mut writer: Box<dyn Write>) -> Result<(), Box<dyn std::error::Error>> {
        let snapshot = Snapshot {
            swacnt: self.swacnt,
            swbcnt: self.swbcnt,
            intim: self.intim,
            instat: self.instat,
            tim1t: self.tim1t,
            tim8t: self.tim8t,
            tim64t: self.tim64t,
            t1024t: self.t1024t,
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
                self.swacnt = snapshot.swacnt;
                self.swbcnt = snapshot.swbcnt;
                self.intim = snapshot.intim;
                self.instat = snapshot.instat;
                self.tim1t = snapshot.tim1t;
                self.tim8t = snapshot.tim8t;
                self.tim64t = snapshot.tim64t;
                self.t1024t = snapshot.t1024t;

                Ok(())
            }
            _ => Err(format!("Unsupported snapshot version: {version}").into()),
        }
    }

    fn memory_read(
        &self,
        address: Address,
        _address_space: AddressSpaceId,
        avoid_side_effects: bool,
        buffer: &mut [u8],
    ) -> Result<(), MemoryError> {
        for (address, buffer_section) in
            RangeInclusive::from_start_and_length(address, buffer.len()).zip(buffer.iter_mut())
        {
            let adjusted_address = address - self.config.registers_assigned_address;

            match adjusted_address {
                0x0 | 0x2 if avoid_side_effects => {
                    return Err(MemoryError(
                        std::iter::once((
                            address..=(address + (buffer.len() - 1)),
                            MemoryErrorType::Denied,
                        ))
                        .collect(),
                    ));
                }
                0x0 if self.swacnt => {
                    *buffer_section = self
                        .swcha
                        .as_ref()
                        .map_or(0, |handler| handler.read_register());
                }
                0x1 => {
                    *buffer_section = u8::from(self.swacnt);
                }
                0x2 if self.swbcnt => {
                    *buffer_section = self
                        .swchb
                        .as_ref()
                        .map_or(0, |handler| handler.read_register());
                }
                0x3 => {
                    *buffer_section = u8::from(self.swbcnt);
                }
                0x4 => {
                    *buffer_section = self.intim;
                }
                0x5 => {
                    *buffer_section = self.instat;
                }
                0x14 => {
                    *buffer_section = self.tim1t.0;
                }
                0x15 => {
                    *buffer_section = self.tim8t.value.0;
                }
                0x16 => {
                    *buffer_section = self.tim64t.value.0;
                }
                0x17 => {
                    *buffer_section = self.t1024t.value.0;
                }
                _ => {
                    return Err(MemoryError(
                        std::iter::once((
                            RangeInclusive::from_start_and_length(address, buffer.len()),
                            MemoryErrorType::Denied,
                        ))
                        .collect(),
                    ));
                }
            }
        }

        Ok(())
    }

    fn memory_write(
        &mut self,
        address: Address,
        _address_space: AddressSpaceId,
        buffer: &[u8],
    ) -> Result<(), MemoryError> {
        for (address, buffer_section) in
            RangeInclusive::from_start_and_length(address, buffer.len()).zip(buffer.iter())
        {
            let adjusted_address = address - self.config.registers_assigned_address;

            match adjusted_address {
                0x0 if !self.swacnt => {
                    self.swcha.as_mut().unwrap().write_register(*buffer_section);
                }
                0x1 => {
                    self.swacnt = *buffer_section != 0;
                }
                0x2 if !self.swbcnt => {
                    self.swchb.as_mut().unwrap().write_register(*buffer_section);
                }
                0x3 => {
                    self.swbcnt = *buffer_section != 0;
                }
                0x14 => {
                    self.tim1t = Wrapping(*buffer_section);
                }
                0x15 => {
                    self.tim8t.value = Wrapping(*buffer_section);
                }
                0x16 => {
                    self.tim64t.value = Wrapping(*buffer_section);
                }
                0x17 => {
                    self.t1024t.value = Wrapping(*buffer_section);
                }
                _ => {
                    return Err(MemoryError(
                        std::iter::once((
                            RangeInclusive::from_start_and_length(address, buffer.len()),
                            MemoryErrorType::Denied,
                        ))
                        .collect(),
                    ));
                }
            }
        }

        Ok(())
    }

    fn synchronize(&mut self, mut context: SynchronizationContext) {
        while context.allocate_period(self.config.frequency.recip()) {
            self.time_counter += 1;
            self.tim1t += 1;

            let value = (self.time_counter - self.tim8t.last_updated) / 8;
            self.tim8t.last_updated += value * 8;
            self.tim8t.value += value as u8;

            let value = (self.time_counter - self.tim64t.last_updated) / 64;
            self.tim64t.last_updated += value * 64;
            self.tim64t.value += value as u8;

            let value = (self.time_counter - self.t1024t.last_updated) / 1024;
            self.t1024t.last_updated += value * 1024;
            self.t1024t.value += value as u8;
        }
    }

    fn needs_work(&self, delta: Period) -> bool {
        delta >= self.config.frequency.recip()
    }
}

impl<P: Platform> ComponentConfig<P> for Mos6532RiotConfig {
    type Component = Mos6532Riot;

    fn build_component(
        self,
        component_builder: ComponentBuilder<'_, P, Self::Component>,
    ) -> Result<Self::Component, Box<dyn std::error::Error>> {
        let ram_assigned_addresses =
            self.ram_assigned_address..=self.ram_assigned_address.checked_add(0x7f).unwrap();

        let component_builder = component_builder
            .memory_map_component(
                self.assigned_address_space,
                self.registers_assigned_address..=self.registers_assigned_address + 0x1f,
            )
            .set_scheduler_participation(SchedulerParticipation::OnDemand);

        component_builder.insert_child_component(
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

        Ok(Self::Component {
            swcha: None,
            swchb: None,
            swacnt: false,
            swbcnt: false,
            intim: 0,
            instat: 0,
            tim1t: Wrapping(0),
            tim8t: Counter::default(),
            tim64t: Counter::default(),
            t1024t: Counter::default(),
            time_counter: 0,
            config: self,
        })
    }
}

#[derive(Debug)]
pub struct Mos6532RiotConfig {
    pub frequency: Frequency,
    pub registers_assigned_address: Address,
    pub ram_assigned_address: Address,
    pub assigned_address_space: AddressSpaceId,
}
