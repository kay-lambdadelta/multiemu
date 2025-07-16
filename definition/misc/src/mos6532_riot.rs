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
        atomic::{AtomicBool, AtomicU8, Ordering},
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
    swacnt: AtomicBool,
    swbcnt: AtomicBool,
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
        self.registers.swacnt.store(false, Ordering::Release);
        self.registers.swbcnt.store(false, Ordering::Release);
        self.registers.intim.store(0, Ordering::Release);
        self.registers.instat.store(0, Ordering::Release);
        self.registers.tim1t.store(0, Ordering::Release);
        self.registers.tim8t.store(0, Ordering::Release);
        self.registers.tim64t.store(0, Ordering::Release);
        self.registers.t1024t.store(0, Ordering::Release);

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
            let adjusted_address = address - self.config.registers_assigned_address;

            match adjusted_address {
                0x0 if self.registers.swacnt.load(Ordering::Acquire) => {
                    *buffer_section = self.registers.swcha.get().unwrap().read_register();
                }
                0x1 => {
                    *buffer_section = if self.registers.swacnt.load(Ordering::Acquire) {
                        1
                    } else {
                        0
                    };
                }
                0x2 if self.registers.swbcnt.load(Ordering::Acquire) => {
                    *buffer_section = self.registers.swchb.get().unwrap().read_register();
                }
                0x3 => {
                    *buffer_section = if self.registers.swbcnt.load(Ordering::Acquire) {
                        1
                    } else {
                        0
                    };
                }
                0x4 => {
                    *buffer_section = self.registers.intim.load(Ordering::Acquire);
                }
                0x5 => {
                    *buffer_section = self.registers.instat.load(Ordering::Acquire);
                }
                0x14 => {
                    *buffer_section = self.registers.tim1t.load(Ordering::Acquire);
                }
                0x15 => {
                    *buffer_section = self.registers.tim8t.load(Ordering::Acquire);
                }
                0x16 => {
                    *buffer_section = self.registers.tim64t.load(Ordering::Acquire);
                }
                0x17 => {
                    *buffer_section = self.registers.t1024t.load(Ordering::Acquire);
                }
                _ => {
                    return Err(MemoryOperationError::from(RangeInclusiveMap::from_iter([
                        (
                            address..=(address + (buffer.len() - 1)),
                            ReadMemoryRecord::Denied,
                        ),
                    ])));
                }
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
        for (address, buffer_section) in
            (address..=(address + (buffer.len() - 1))).zip(buffer.iter_mut())
        {
            let adjusted_address = address - self.config.registers_assigned_address;

            match adjusted_address {
                0x0 | 0x2 => {
                    return Err(MemoryOperationError::from(RangeInclusiveMap::from_iter([
                        (
                            address..=(address + (buffer.len() - 1)),
                            PreviewMemoryRecord::Impossible,
                        ),
                    ])));
                }
                0x1 => {
                    *buffer_section = if self.registers.swacnt.load(Ordering::Acquire) {
                        1
                    } else {
                        0
                    };
                }
                0x3 => {
                    *buffer_section = if self.registers.swbcnt.load(Ordering::Acquire) {
                        1
                    } else {
                        0
                    };
                }
                0x4 => {
                    *buffer_section = self.registers.intim.load(Ordering::Acquire);
                }
                0x5 => {
                    *buffer_section = self.registers.instat.load(Ordering::Acquire);
                }
                0x14 => {
                    *buffer_section = self.registers.tim1t.load(Ordering::Acquire);
                }
                0x15 => {
                    *buffer_section = self.registers.tim8t.load(Ordering::Acquire);
                }
                0x16 => {
                    *buffer_section = self.registers.tim64t.load(Ordering::Acquire);
                }
                0x17 => {
                    *buffer_section = self.registers.t1024t.load(Ordering::Acquire);
                }
                _ => {
                    return Err(MemoryOperationError::from(RangeInclusiveMap::from_iter([
                        (
                            address..=(address + (buffer.len() - 1)),
                            PreviewMemoryRecord::Denied,
                        ),
                    ])));
                }
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
            let adjusted_address = address - self.config.registers_assigned_address;

            match adjusted_address {
                0x0 if !self.registers.swacnt.load(Ordering::Acquire) => {
                    self.registers
                        .swcha
                        .get()
                        .unwrap()
                        .write_register(*buffer_section);
                }
                0x1 => {
                    self.registers.swacnt.store(
                        if *buffer_section == 0 { false } else { true },
                        Ordering::Release,
                    );
                }
                0x2 if !self.registers.swbcnt.load(Ordering::Acquire) => {
                    self.registers
                        .swchb
                        .get()
                        .unwrap()
                        .write_register(*buffer_section);
                }
                0x3 => {
                    self.registers.swbcnt.store(
                        if *buffer_section == 0 { false } else { true },
                        Ordering::Release,
                    );
                }
                0x14 => {
                    self.registers
                        .tim1t
                        .store(*buffer_section, Ordering::Release);
                }
                0x15 => {
                    self.registers
                        .tim8t
                        .store(*buffer_section, Ordering::Release);
                }
                0x16 => {
                    self.registers
                        .tim64t
                        .store(*buffer_section, Ordering::Release);
                }
                0x17 => {
                    self.registers
                        .t1024t
                        .store(*buffer_section, Ordering::Release);
                }
                _ => {
                    return Err(MemoryOperationError::from(RangeInclusiveMap::from_iter([
                        (
                            address..=(address + (buffer.len() - 1)),
                            WriteMemoryRecord::Denied,
                        ),
                    ])));
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
        component_ref: ComponentRef<Self::Component>,
        component_builder: ComponentBuilder<'_, P, Self::Component>,
    ) {
        let registers = Registers {
            swcha: OnceLock::new(),
            swchb: OnceLock::new(),
            swacnt: AtomicBool::new(false),
            swbcnt: AtomicBool::new(false),
            intim: AtomicU8::new(0),
            instat: AtomicU8::new(0),
            tim1t: AtomicU8::new(0),
            tim8t: AtomicU8::new(0),
            tim64t: AtomicU8::new(0),
            t1024t: AtomicU8::new(0),
        };

        let component_builder = component_builder.map_memory([(
            self.assigned_address_space,
            self.registers_assigned_address..=self.registers_assigned_address + 0x1f,
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
                                Ordering::Acquire,
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
                                Ordering::Acquire,
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
                                Ordering::Acquire,
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
                                Ordering::Acquire,
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
