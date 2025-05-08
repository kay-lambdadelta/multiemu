use multiemu_machine::{
    builder::ComponentBuilder,
    component::{Component, FromConfig, RuntimeEssentials},
    memory::{
        AddressSpaceHandle,
        callbacks::{ReadMemory, WriteMemory},
        memory_translation_table::{PreviewMemoryRecord, ReadMemoryRecord, WriteMemoryRecord},
    },
};
use num::rational::Ratio;
use rangemap::RangeInclusiveMap;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::{
    fmt::Debug,
    io::{Read, Write},
    num::NonZero,
    sync::{
        Arc, OnceLock, RwLock,
        atomic::{AtomicU8, Ordering},
    },
};
use versions::SemVer;

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

pub struct Mos6532Riot {
    ram: Arc<RwLock<[u8; 128]>>,
    registers: Arc<Registers>,
}

impl Mos6532Riot {
    pub fn install_swcha(&self, callback: impl SwchaCallback) {
        if self.registers.swcha.set(Box::new(callback)).is_err() {
            panic!("SWCHA already set");
        }
    }

    pub fn install_swchb(&self, callback: impl SwchbCallback) {
        if self.registers.swchb.set(Box::new(callback)).is_err() {
            panic!("SWCHB already set");
        }
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

    fn save(&self, mut entry: &mut dyn Write) -> Result<SemVer, Box<dyn std::error::Error>> {
        let snapshot = Snapshot {
            swacnt: self.registers.swacnt.load(Ordering::Relaxed),
            swbcnt: self.registers.swbcnt.load(Ordering::Relaxed),
            intim: self.registers.intim.load(Ordering::Relaxed),
            instat: self.registers.instat.load(Ordering::Relaxed),
            tim1t: self.registers.tim1t.load(Ordering::Relaxed),
            tim8t: self.registers.tim8t.load(Ordering::Relaxed),
            tim64t: self.registers.tim64t.load(Ordering::Relaxed),
            t1024t: self.registers.t1024t.load(Ordering::Relaxed),
            ram: *self.ram.read().unwrap(),
        };

        bincode::serde::encode_into_std_write(snapshot, &mut entry, bincode::config::standard())?;

        Ok(SemVer::new("1.0.0").unwrap())
    }

    fn load(
        &self,
        mut entry: &mut dyn Read,
        version: SemVer,
    ) -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            version,
            SemVer::new("1.0.0").unwrap(),
            "Incompatible snapshot version"
        );

        let snapshot: Snapshot =
            bincode::serde::decode_from_std_read(&mut entry, bincode::config::standard())?;

        self.registers
            .swacnt
            .store(snapshot.swacnt, Ordering::Relaxed);
        self.registers
            .swbcnt
            .store(snapshot.swbcnt, Ordering::Relaxed);
        self.registers
            .intim
            .store(snapshot.intim, Ordering::Relaxed);
        self.registers
            .instat
            .store(snapshot.instat, Ordering::Relaxed);
        self.registers
            .tim1t
            .store(snapshot.tim1t, Ordering::Relaxed);
        self.registers
            .tim8t
            .store(snapshot.tim8t, Ordering::Relaxed);
        self.registers
            .tim64t
            .store(snapshot.tim64t, Ordering::Relaxed);
        self.registers
            .t1024t
            .store(snapshot.t1024t, Ordering::Relaxed);
        self.ram.write().unwrap().copy_from_slice(&snapshot.ram);

        Ok(())
    }
}

impl FromConfig for Mos6532Riot {
    type Config = M6532RiotConfig;
    type Quirks = ();

    fn from_config(
        component_builder: ComponentBuilder<Self>,
        essentials: Arc<RuntimeEssentials>,
        config: Self::Config,
        _quirks: Self::Quirks,
    ) {
        let config = Arc::new(config);
        let ram = Arc::new(RwLock::new([0; 128]));
        let memory_callbacks = Arc::new(RamMemoryCallbacks {
            config: config.clone(),
            ram: ram.clone(),
        });
        let registers = Arc::new(Registers {
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
        });

        let assigned_ranges = [(
            config.assigned_address_space,
            config.ram_assigned_address..=config.ram_assigned_address + 127,
        )];

        essentials
            .memory_translation_table
            .insert_memory(memory_callbacks.clone(), assigned_ranges);

        let swcha = Arc::new(SwchaMemoryCallback {
            registers: registers.clone(),
        });

        essentials.memory_translation_table.insert_memory(
            swcha,
            [(
                config.assigned_address_space,
                config.registers_assigned_address..=config.registers_assigned_address,
            )],
        );

        let swchb = Arc::new(SwchbMemoryCallback {
            registers: registers.clone(),
        });

        essentials.memory_translation_table.insert_memory(
            swchb,
            [(
                config.assigned_address_space,
                config.registers_assigned_address + 1..=config.registers_assigned_address + 1,
            )],
        );

        let component_builder = {
            // Make the timers operate
            component_builder
                .insert_task(config.frequency, {
                    let registers = registers.clone();

                    move |_: &Self, period: NonZero<_>| {
                        registers
                            .tim1t
                            .fetch_add(period.get() as u8, Ordering::Relaxed);
                    }
                })
                .insert_task(config.frequency / 8, {
                    let registers = registers.clone();

                    move |_: &Self, period: NonZero<_>| {
                        registers
                            .tim8t
                            .fetch_add(period.get() as u8, Ordering::Relaxed);
                    }
                })
                .insert_task(config.frequency / 64, {
                    let registers = registers.clone();

                    move |_: &Self, period: NonZero<_>| {
                        registers
                            .tim64t
                            .fetch_add(period.get() as u8, Ordering::Relaxed);
                    }
                })
                .insert_task(config.frequency / 1024, {
                    let registers = registers.clone();

                    move |_: &Self, period: NonZero<_>| {
                        registers
                            .t1024t
                            .fetch_add(period.get() as u8, Ordering::Relaxed);
                    }
                })
        };

        component_builder.build_global(Self { ram, registers });
    }
}

#[derive(Debug)]
pub struct M6532RiotConfig {
    pub frequency: Ratio<u32>,
    pub ram_assigned_address: usize,
    pub registers_assigned_address: usize,
    pub assigned_address_space: AddressSpaceHandle,
}

#[derive(Debug)]
struct RamMemoryCallbacks {
    config: Arc<M6532RiotConfig>,
    ram: Arc<RwLock<[u8; 128]>>,
}

impl ReadMemory for RamMemoryCallbacks {
    fn read_memory(
        &self,
        address: usize,
        _address_space: AddressSpaceHandle,
        buffer: &mut [u8],
        _errors: &mut RangeInclusiveMap<usize, ReadMemoryRecord>,
    ) {
        let memory = self.ram.read().unwrap();
        let adjusted_offset = address - self.config.ram_assigned_address;

        buffer.copy_from_slice(&memory[adjusted_offset..=(adjusted_offset + (buffer.len() - 1))]);
    }

    fn preview_memory(
        &self,
        address: usize,
        _address_space: AddressSpaceHandle,
        buffer: &mut [u8],
        _errors: &mut RangeInclusiveMap<usize, PreviewMemoryRecord>,
    ) {
        let memory = self.ram.read().unwrap();
        let adjusted_offset = address - self.config.ram_assigned_address;

        buffer.copy_from_slice(&memory[adjusted_offset..=(adjusted_offset + (buffer.len() - 1))]);
    }
}

impl WriteMemory for RamMemoryCallbacks {
    fn write_memory(
        &self,
        address: usize,
        _address_space: AddressSpaceHandle,
        buffer: &[u8],
        _errors: &mut RangeInclusiveMap<usize, WriteMemoryRecord>,
    ) {
        let mut memory = self.ram.write().unwrap();
        let adjusted_offset = address - self.config.ram_assigned_address;

        memory[adjusted_offset..=(adjusted_offset + (buffer.len() - 1))].copy_from_slice(buffer);
    }
}

#[derive(Debug)]
struct SwchaMemoryCallback {
    registers: Arc<Registers>,
}

impl ReadMemory for SwchaMemoryCallback {
    fn read_memory(
        &self,
        _address: usize,
        _address_space: AddressSpaceHandle,
        buffer: &mut [u8],
        _errors: &mut RangeInclusiveMap<usize, ReadMemoryRecord>,
    ) {
        buffer[0] = self.registers.swcha.get().unwrap().read_memory();
    }
}

impl WriteMemory for SwchaMemoryCallback {
    fn write_memory(
        &self,
        _address: usize,
        _address_space: AddressSpaceHandle,
        buffer: &[u8],
        _errors: &mut RangeInclusiveMap<usize, WriteMemoryRecord>,
    ) {
        self.registers.swcha.get().unwrap().write_memory(buffer[0]);
    }
}

#[derive(Debug)]
struct SwchbMemoryCallback {
    registers: Arc<Registers>,
}

impl ReadMemory for SwchbMemoryCallback {
    fn read_memory(
        &self,
        _address: usize,
        _address_space: AddressSpaceHandle,
        buffer: &mut [u8],
        _errors: &mut RangeInclusiveMap<usize, ReadMemoryRecord>,
    ) {
        buffer[0] = self.registers.swchb.get().unwrap().read_memory();
    }
}

impl WriteMemory for SwchbMemoryCallback {
    fn write_memory(
        &self,
        _address: usize,
        _address_space: AddressSpaceHandle,
        buffer: &[u8],
        _errors: &mut RangeInclusiveMap<usize, WriteMemoryRecord>,
    ) {
        self.registers.swchb.get().unwrap().write_memory(buffer[0]);
    }
}
