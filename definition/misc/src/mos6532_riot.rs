use multiemu_runtime::{
    builder::ComponentBuilder,
    component::{Component, ComponentConfig, component_ref::ComponentRef},
    memory::{
        Address,
        callbacks::{Memory, ReadMemory, WriteMemory},
        memory_translation_table::{
            MemoryOperationError, PreviewMemoryRecord, ReadMemoryRecord, WriteMemoryRecord,
            address_space::AddressSpaceHandle,
        },
    },
    scheduler::{SchedulerHandle, YieldReason},
};
use num::rational::Ratio;
use rangemap::RangeInclusiveMap;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::{
    fmt::Debug,
    sync::{
        Arc, OnceLock, RwLock,
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
    #[serde_as(as = "[_; 128]")]
    ram: [u8; 128],
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
    fn read_memory(&self) -> u8;

    fn write_memory(&self, value: u8);
}

pub trait SwchbCallback: Debug + Send + Sync + 'static {
    fn read_memory(&self) -> u8;

    fn write_memory(&self, value: u8);
}

#[derive(Debug)]
pub struct Mos6532Riot {
    ram: Arc<RwLock<[u8; 128]>>,
    registers: Registers,
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
    fn reset(&self) {
        self.ram.write().unwrap().fill(0);

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
}

impl<B: ComponentBuilder<Component = Mos6532Riot>> ComponentConfig<B> for Mos6532RiotConfig {
    type Component = Mos6532Riot;

    fn build_component(
        self,
        component_ref: ComponentRef<Self::Component>,
        component_builder: B,
    ) -> B::BuildOutput {
        let config = Arc::new(self);
        let ram = Arc::new(RwLock::new([0; 128]));
        let memory_callbacks = Arc::new(RamMemoryCallbacks {
            config: config.clone(),
            ram: ram.clone(),
        });
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

        let assigned_ranges = [(
            config.assigned_address_space,
            config.ram_assigned_address..=config.ram_assigned_address + 127,
        )];

        let (component_builder, _) =
            component_builder.insert_memory(memory_callbacks.clone(), assigned_ranges);

        let swcha = Arc::new(SwchaMemoryCallback {
            component: component_ref.clone(),
        });

        let (component_builder, _) = component_builder.insert_memory(
            swcha,
            [(
                config.assigned_address_space,
                config.registers_assigned_address..=config.registers_assigned_address,
            )],
        );

        let swchb = Arc::new(SwchbMemoryCallback {
            component: component_ref.clone(),
        });

        let (component_builder, _) = component_builder.insert_memory(
            swchb,
            [(
                config.assigned_address_space,
                config.registers_assigned_address + 1..=config.registers_assigned_address + 1,
            )],
        );

        let component_builder = set_up_timer_tasks(component_ref, config, component_builder);

        component_builder.build_global(Self::Component { ram, registers })
    }
}

fn set_up_timer_tasks<B: ComponentBuilder<Component = Mos6532Riot>>(
    component_ref: ComponentRef<Mos6532Riot>,
    config: Arc<Mos6532RiotConfig>,
    component_builder: B,
) -> B {
    {
        // Make the timers operate
        component_builder
            .insert_task(config.frequency, {
                let component_ref = component_ref.clone();

                move |mut handle: SchedulerHandle| {
                    let mut should_exit = false;

                    while !should_exit {
                        component_ref
                            .interact(|component| {
                                component.registers.tim1t.fetch_add(1, Ordering::Relaxed)
                            })
                            .unwrap();

                        handle.tick(|reason| {
                            if reason == YieldReason::Exit {
                                should_exit = true;
                            }
                        });
                    }
                }
            })
            .insert_task(config.frequency / 8, {
                let component_ref = component_ref.clone();

                move |mut handle: SchedulerHandle| {
                    let mut should_exit = false;

                    while !should_exit {
                        component_ref
                            .interact(|component| {
                                component.registers.tim8t.fetch_add(1, Ordering::Relaxed)
                            })
                            .unwrap();

                        handle.tick(|reason| {
                            if reason == YieldReason::Exit {
                                should_exit = true;
                            }
                        });
                    }
                }
            })
            .insert_task(config.frequency / 64, {
                let component_ref = component_ref.clone();

                move |mut handle: SchedulerHandle| {
                    let mut should_exit = false;

                    while !should_exit {
                        component_ref
                            .interact(|component| {
                                component.registers.tim64t.fetch_add(1, Ordering::Relaxed)
                            })
                            .unwrap();

                        handle.tick(|reason| {
                            if reason == YieldReason::Exit {
                                should_exit = true;
                            }
                        });
                    }
                }
            })
            .insert_task(config.frequency / 1024, {
                let component_ref = component_ref.clone();

                move |mut handle: SchedulerHandle| {
                    let mut should_exit = false;

                    while !should_exit {
                        component_ref
                            .interact(|component| {
                                component.registers.t1024t.fetch_add(1, Ordering::Relaxed)
                            })
                            .unwrap();

                        handle.tick(|reason| {
                            if reason == YieldReason::Exit {
                                should_exit = true;
                            }
                        });
                    }
                }
            })
    }
}

#[derive(Debug)]
pub struct Mos6532RiotConfig {
    pub frequency: Ratio<u32>,
    pub ram_assigned_address: Address,
    pub registers_assigned_address: Address,
    pub assigned_address_space: AddressSpaceHandle,
}

#[derive(Debug)]
struct RamMemoryCallbacks {
    config: Arc<Mos6532RiotConfig>,
    ram: Arc<RwLock<[u8; 128]>>,
}

impl Memory for RamMemoryCallbacks {}

impl ReadMemory for RamMemoryCallbacks {
    fn read_memory(
        &self,
        address: Address,
        _address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), MemoryOperationError<ReadMemoryRecord>> {
        let memory = self.ram.read().unwrap();
        let adjusted_offset = address - self.config.ram_assigned_address;

        buffer.copy_from_slice(&memory[adjusted_offset..=(adjusted_offset + (buffer.len() - 1))]);

        Ok(())
    }

    fn preview_memory(
        &self,
        address: Address,
        _address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), MemoryOperationError<PreviewMemoryRecord>> {
        // The installed callbacks might have side effects

        Err(MemoryOperationError::from(RangeInclusiveMap::from_iter([
            (
                address..=(address + (buffer.len() - 1)),
                PreviewMemoryRecord::Impossible,
            ),
        ])))
    }
}

impl WriteMemory for RamMemoryCallbacks {
    fn write_memory(
        &self,
        address: Address,
        _address_space: AddressSpaceHandle,
        buffer: &[u8],
    ) -> Result<(), MemoryOperationError<WriteMemoryRecord>> {
        let mut memory = self.ram.write().unwrap();
        let adjusted_offset = address - self.config.ram_assigned_address;

        memory[adjusted_offset..=(adjusted_offset + (buffer.len() - 1))].copy_from_slice(buffer);

        Ok(())
    }
}

#[derive(Debug)]
struct SwchaMemoryCallback {
    component: ComponentRef<Mos6532Riot>,
}

impl Memory for SwchaMemoryCallback {}

impl ReadMemory for SwchaMemoryCallback {
    fn read_memory(
        &self,
        _address: Address,
        _address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), MemoryOperationError<ReadMemoryRecord>> {
        buffer[0] = self
            .component
            .interact(|component| component.registers.swcha.get().unwrap().read_memory())
            .unwrap();

        Ok(())
    }
}

impl WriteMemory for SwchaMemoryCallback {
    fn write_memory(
        &self,
        _address: Address,
        _address_space: AddressSpaceHandle,
        buffer: &[u8],
    ) -> Result<(), MemoryOperationError<WriteMemoryRecord>> {
        self.component
            .interact(|component| {
                component
                    .registers
                    .swcha
                    .get()
                    .unwrap()
                    .write_memory(buffer[0])
            })
            .unwrap();

        Ok(())
    }
}

#[derive(Debug)]
struct SwchbMemoryCallback {
    component: ComponentRef<Mos6532Riot>,
}

impl Memory for SwchbMemoryCallback {}

impl ReadMemory for SwchbMemoryCallback {
    fn read_memory(
        &self,
        _address: Address,
        _address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), MemoryOperationError<ReadMemoryRecord>> {
        buffer[0] = self
            .component
            .interact(|component| component.registers.swchb.get().unwrap().read_memory())
            .unwrap();

        Ok(())
    }
}

impl WriteMemory for SwchbMemoryCallback {
    fn write_memory(
        &self,
        _address: Address,
        _address_space: AddressSpaceHandle,
        buffer: &[u8],
    ) -> Result<(), MemoryOperationError<WriteMemoryRecord>> {
        self.component
            .interact(|component| {
                component
                    .registers
                    .swchb
                    .get()
                    .unwrap()
                    .write_memory(buffer[0])
            })
            .unwrap();

        Ok(())
    }
}
