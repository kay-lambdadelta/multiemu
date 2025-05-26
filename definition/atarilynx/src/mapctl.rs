use deku::{DekuContainerRead, DekuContainerWrite, DekuRead, DekuWrite};
use multiemu_machine::{
    builder::ComponentBuilder,
    component::{Component, ComponentConfig},
    display::backend::RenderApi,
    memory::{
        callbacks::{ReadMemory, WriteMemory},
        memory_translation_table::{
            MemoryHandle, MemoryOperationError, ReadMemoryRecord, RemapCallback, WriteMemoryRecord,
            address_space::AddressSpaceHandle,
        },
    },
};
use rangemap::RangeInclusiveMap;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub struct Mapctl(Arc<Mutex<MapctlStatus>>);

impl Component for Mapctl {}

#[derive(Debug)]
pub struct MapctlConfig {
    pub ram_memory_handle: MemoryHandle,
    pub suzy_memory_handle: MemoryHandle,
    pub mikey_memory_handle: MemoryHandle,
}

impl<R: RenderApi> ComponentConfig<R> for MapctlConfig {
    type Component = Mapctl;

    fn build_component(self, component_builder: ComponentBuilder<R, Self::Component>) {
        let register = Arc::new(Mutex::new(MapctlStatus::default()));

        component_builder.build_global(Mapctl(register));
    }
}

#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize, DekuRead, DekuWrite, Default)]
pub struct MapctlStatus {
    #[deku(bits = 1)]
    suzy: bool,
    #[deku(bits = 1)]
    mikey: bool,
    #[deku(bits = 1)]
    rom: bool,
    #[deku(bits = 1)]
    vector: bool,
    #[deku(bits = 3)]
    reserved: u8,
    #[deku(bits = 1)]
    sequential_disable: bool,
}

#[derive(Debug)]
struct MemoryCallbacks {
    registers: Arc<Mutex<MapctlStatus>>,
    ram_memory_handle: MemoryHandle,
    suzy_memory_handle: MemoryHandle,
    mikey_memory_handle: MemoryHandle,
}

impl ReadMemory for MemoryCallbacks {
    fn read_memory(
        &self,
        _address: usize,
        _address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), MemoryOperationError<ReadMemoryRecord>> {
        let register_guard = self.registers.lock().unwrap();
        register_guard.to_slice(buffer).unwrap();

        Ok(())
    }
}

impl WriteMemory for MemoryCallbacks {
    fn write_memory(
        &self,
        _address: usize,
        address_space: AddressSpaceHandle,
        buffer: &[u8],
    ) -> Result<(), MemoryOperationError<WriteMemoryRecord>> {
        let mut register_guard = self.registers.lock().unwrap();
        let register = MapctlStatus::from_bytes((buffer, 0)).unwrap().1;
        *register_guard = register;

        Err(MemoryOperationError {
            records: RangeInclusiveMap::default(),
            remap_callback: Some(RemapCallback::new(|memory_translation_table| {})),
        })
    }
}
