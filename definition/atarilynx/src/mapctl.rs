use crate::{
    MAPCTL_ADDRESS, MIKEY_ADDRESSES, RESERVED_MEMORY_ADDRESS, SUZY_ADDRESSES, VECTOR_ADDRESSES,
};
use deku::{DekuContainerRead, DekuContainerWrite, DekuRead, DekuWrite};
use multiemu_runtime::{
    builder::ComponentBuilder,
    component::{Component, ComponentConfig, component_ref::ComponentRef},
    memory::{
        Address,
        callbacks::{Memory, ReadMemory, WriteMemory},
        memory_translation_table::{
            MemoryHandle, MemoryOperationError, ReadMemoryRecord, RemapCallback, WriteMemoryRecord,
            address_space::AddressSpaceHandle,
        },
    },
};
use rangemap::RangeInclusiveMap;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Debug)]
pub struct Mapctl(Arc<Mutex<MapctlStatus>>);

impl Component for Mapctl {}

#[derive(Debug)]
pub struct MapctlConfig {
    pub ram_memory_handle: MemoryHandle,
    pub suzy_memory_handle: MemoryHandle,
    pub mikey_memory_handle: MemoryHandle,
    pub vector_memory_handle: MemoryHandle,
    pub reserved_memory_handle: MemoryHandle,
    pub cpu_address_space: AddressSpaceHandle,
}

impl<B: ComponentBuilder<Component = Mapctl>> ComponentConfig<B> for MapctlConfig {
    type Component = Mapctl;

    fn build_component(
        self,
        _component_ref: ComponentRef<Self::Component>,
        component_builder: B,
    ) -> B::BuildOutput {
        let register = Arc::new(Mutex::new(MapctlStatus::default()));

        let (component_builder, _) = component_builder.insert_memory(
            MemoryCallbacks {
                cpu_address_space: self.cpu_address_space,
                ram_memory_handle: self.ram_memory_handle,
                suzy_memory_handle: self.suzy_memory_handle,
                mikey_memory_handle: self.mikey_memory_handle,
                vector_memory_handle: self.vector_memory_handle,
                reserved_memory_handle: self.reserved_memory_handle,
                my_memory_handle: OnceLock::new(),
                registers: register.clone(),
            },
            [(self.cpu_address_space, 0xfff9..=0xfff9)],
        );

        component_builder.build_global(Mapctl(register))
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
    cpu_address_space: AddressSpaceHandle,
    ram_memory_handle: MemoryHandle,
    suzy_memory_handle: MemoryHandle,
    mikey_memory_handle: MemoryHandle,
    vector_memory_handle: MemoryHandle,
    reserved_memory_handle: MemoryHandle,
    my_memory_handle: OnceLock<MemoryHandle>,
}

impl Memory for MemoryCallbacks {
    fn set_memory_handle(&self, handle: MemoryHandle) {
        self.my_memory_handle.set(handle).unwrap();
    }
}

impl ReadMemory for MemoryCallbacks {
    fn read_memory(
        &self,
        _address: Address,
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
        _address: Address,
        _address_space: AddressSpaceHandle,
        buffer: &[u8],
    ) -> Result<(), MemoryOperationError<WriteMemoryRecord>> {
        let mut register_guard = self.registers.lock().unwrap();
        let register = MapctlStatus::from_bytes((buffer, 0)).unwrap().1;
        *register_guard = register;

        Err(MemoryOperationError {
            records: RangeInclusiveMap::default(),
            remap_callback: {
                let ram_memory_handle = self.ram_memory_handle;
                let suzy_memory_handle = self.suzy_memory_handle;
                let mikey_memory_handle = self.mikey_memory_handle;
                let reserved_memory_handle = self.reserved_memory_handle;
                let vector_memory_handle = self.vector_memory_handle;
                let my_memory_handle = *self.my_memory_handle.get().unwrap();
                let cpu_address_space = self.cpu_address_space;

                Some(RemapCallback::new(move |memory_translation_table| {
                    // remap ram
                    memory_translation_table.remap_memory(
                        ram_memory_handle,
                        cpu_address_space,
                        [0x0000..=0xffff],
                    );

                    // optionally remap the other stuff on top of it
                    if register.suzy {
                        memory_translation_table.remap_memory(
                            suzy_memory_handle,
                            cpu_address_space,
                            [SUZY_ADDRESSES],
                        );
                    }

                    if register.mikey {
                        memory_translation_table.remap_memory(
                            mikey_memory_handle,
                            cpu_address_space,
                            [MIKEY_ADDRESSES],
                        );
                    }

                    if register.vector {
                        memory_translation_table.remap_memory(
                            vector_memory_handle,
                            cpu_address_space,
                            [VECTOR_ADDRESSES],
                        );
                    }

                    // http://www.monlynx.de/lynx/hardware.html

                    memory_translation_table.remap_memory(
                        reserved_memory_handle,
                        cpu_address_space,
                        [RESERVED_MEMORY_ADDRESS..=RESERVED_MEMORY_ADDRESS],
                    );

                    memory_translation_table.remap_memory(
                        my_memory_handle,
                        cpu_address_space,
                        [MAPCTL_ADDRESS..=MAPCTL_ADDRESS],
                    );
                }))
            },
        })
    }
}
