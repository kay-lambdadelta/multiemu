use multiemu_runtime::{
    builder::ComponentBuilder,
    component::{Component, ComponentConfig, ComponentRef},
    memory::{
        Address, AddressSpaceHandle, MemoryOperationError, PreviewMemoryRecord, ReadMemoryRecord,
        WriteMemoryRecord,
    },
    platform::Platform,
};
use num::rational::Ratio;
use rangemap::RangeInclusiveMap;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::{
    fmt::Debug,
    num::NonZero,
    sync::{
        OnceLock,
        atomic::{AtomicU8, Ordering},
    },
};

#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct Snapshot {
    swacnt: u8,
    swbcnt: u8,
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
    swacnt: AtomicU8,
    swbcnt: AtomicU8,
    intim: AtomicU8,
    instat: AtomicU8,
    tim1t: AtomicU8,
    tim8t: AtomicU8,
    tim64t: AtomicU8,
    t1024t: AtomicU8,
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
    fn on_reset(&self) {
        self.registers.swacnt.store(0, Ordering::Relaxed);
        self.registers.swbcnt.store(0, Ordering::Relaxed);
        self.registers.intim.store(0, Ordering::Relaxed);
        self.registers.instat.store(0, Ordering::Relaxed);
        self.registers.tim1t.store(0, Ordering::Relaxed);
        self.registers.tim8t.store(0, Ordering::Relaxed);
        self.registers.tim64t.store(0, Ordering::Relaxed);
        self.registers.t1024t.store(0, Ordering::Relaxed);

        // I dunno what to do with the handlers
        // The components that installed the handlers will be reset too so its probably fine
    }

    fn read_memory(
        &self,
        address: Address,
        _address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), MemoryOperationError<ReadMemoryRecord>> {
        for (address, buffer_section) in
            (address..=(address + (buffer.len() - 1))).zip(buffer.iter_mut())
        {
            if address == self.config.registers_assigned_address {
                *buffer_section = self.registers.swcha.get().unwrap().read_register();
            } else if address == self.config.registers_assigned_address + 1 {
                *buffer_section = self.registers.swchb.get().unwrap().read_register();
            } else {
                return Err(MemoryOperationError::from(RangeInclusiveMap::from_iter([
                    (
                        address..=(address + (buffer.len() - 1)),
                        ReadMemoryRecord::Denied,
                    ),
                ])));
            }
        }

        Ok(())
    }

    fn preview_memory(
        &self,
        address: Address,
        _address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), MemoryOperationError<PreviewMemoryRecord>> {
        // The installed callbacks might have side effects

        for address in address..=(address + (buffer.len() - 1)) {
            if address == self.config.registers_assigned_address
                || address == self.config.registers_assigned_address + 1
            {
                return Err(MemoryOperationError::from(RangeInclusiveMap::from_iter([
                    (
                        address..=(address + (buffer.len() - 1)),
                        PreviewMemoryRecord::Impossible,
                    ),
                ])));
            } else {
                return Err(MemoryOperationError::from(RangeInclusiveMap::from_iter([
                    (
                        address..=(address + (buffer.len() - 1)),
                        PreviewMemoryRecord::Denied,
                    ),
                ])));
            }
        }

        Ok(())
    }

    fn write_memory(
        &self,
        address: Address,
        _address_space: AddressSpaceHandle,
        buffer: &[u8],
    ) -> Result<(), MemoryOperationError<WriteMemoryRecord>> {
        for (address, buffer_section) in
            (address..=(address + (buffer.len() - 1))).zip(buffer.iter())
        {
            if address == self.config.registers_assigned_address {
                self.registers
                    .swcha
                    .get()
                    .unwrap()
                    .write_register(*buffer_section);
            } else if address == self.config.registers_assigned_address + 1 {
                self.registers
                    .swchb
                    .get()
                    .unwrap()
                    .write_register(*buffer_section);
            } else {
                return Err(MemoryOperationError::from(RangeInclusiveMap::from_iter([
                    (
                        address..=(address + (buffer.len() - 1)),
                        WriteMemoryRecord::Denied,
                    ),
                ])));
            }
        }

        Ok(())
    }
}

impl<P: Platform> ComponentConfig<P> for Mos6532RiotConfig {
    type Component = Mos6532Riot;

    fn build_component(
        self,
        component_ref: ComponentRef<Self::Component>,
        component_builder: ComponentBuilder<'_, P, Self::Component>,
    ) {
        let registers = Registers {
            swcha: OnceLock::new(),
            swchb: OnceLock::new(),
            swacnt: AtomicU8::new(0),
            swbcnt: AtomicU8::new(0),
            intim: AtomicU8::new(0),
            instat: AtomicU8::new(0),
            tim1t: AtomicU8::new(0),
            tim8t: AtomicU8::new(0),
            tim64t: AtomicU8::new(0),
            t1024t: AtomicU8::new(0),
        };

        let component_builder = component_builder.map_memory([(
            self.assigned_address_space,
            self.registers_assigned_address..=self.registers_assigned_address,
        )]);

        let component_builder = component_builder.map_memory([(
            self.assigned_address_space,
            self.registers_assigned_address + 1..=self.registers_assigned_address + 1,
        )]);

        let component_builder = set_up_timer_tasks(component_ref, &self, component_builder);

        component_builder.build_global(Self::Component {
            registers,
            config: self,
        })
    }
}

fn set_up_timer_tasks<'a, P: Platform>(
    component_ref: ComponentRef<Mos6532Riot>,
    config: &Mos6532RiotConfig,
    component_builder: ComponentBuilder<'a, P, Mos6532Riot>,
) -> ComponentBuilder<'a, P, Mos6532Riot> {
    {
        // Make the timers operate
        component_builder
            .insert_lazy_task(config.frequency, {
                let component_ref = component_ref.clone();

                move |slice: NonZero<u32>| {
                    component_ref
                        .interact(|component| {
                            component.registers.tim1t.fetch_add(
                                slice.get().try_into().unwrap_or(u8::MAX),
                                Ordering::Relaxed,
                            )
                        })
                        .unwrap();
                }
            })
            .insert_lazy_task(config.frequency / 8, {
                let component_ref = component_ref.clone();

                move |slice: NonZero<u32>| {
                    component_ref
                        .interact(|component| {
                            component.registers.tim8t.fetch_add(
                                slice.get().try_into().unwrap_or(u8::MAX),
                                Ordering::Relaxed,
                            )
                        })
                        .unwrap();
                }
            })
            .insert_lazy_task(config.frequency / 64, {
                let component_ref = component_ref.clone();

                move |slice: NonZero<u32>| {
                    component_ref
                        .interact(|component| {
                            component.registers.tim64t.fetch_add(
                                slice.get().try_into().unwrap_or(u8::MAX),
                                Ordering::Relaxed,
                            )
                        })
                        .unwrap();
                }
            })
            .insert_lazy_task(config.frequency / 1024, {
                let component_ref = component_ref.clone();

                move |slice: NonZero<u32>| {
                    component_ref
                        .interact(|component| {
                            component.registers.t1024t.fetch_add(
                                slice.get().try_into().unwrap_or(u8::MAX),
                                Ordering::Relaxed,
                            )
                        })
                        .unwrap();
                }
            })
    }
}

#[derive(Debug)]
pub struct Mos6532RiotConfig {
    pub frequency: Ratio<u32>,
    pub registers_assigned_address: Address,
    pub assigned_address_space: AddressSpaceHandle,
}
