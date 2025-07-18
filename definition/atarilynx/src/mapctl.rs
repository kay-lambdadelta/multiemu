use crate::{
    MAPCTL_ADDRESS, MIKEY_ADDRESSES, RESERVED_MEMORY_ADDRESS, SUZY_ADDRESSES, VECTOR_ADDRESSES,
};
use deku::{DekuContainerRead, DekuContainerWrite, DekuRead, DekuWrite};
use multiemu_runtime::{
    builder::ComponentBuilder,
    component::{BuildError, Component, ComponentConfig, ComponentId, ComponentRef},
    memory::{
        Address, AddressSpaceHandle, MemoryOperationError, ReadMemoryRecord, RemapCallback,
        WriteMemoryRecord,
    },
    platform::Platform,
};
use multiemu_save::ComponentSave;
use rangemap::RangeInclusiveMap;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Debug)]
pub struct Mapctl {
    config: MapctlConfig,
    status: Mutex<MapctlStatus>,
    my_id: ComponentId,
}

impl Component for Mapctl {
    fn read_memory(
        &self,
        _address: Address,
        _address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), MemoryOperationError<ReadMemoryRecord>> {
        let register_guard = self.status.lock().unwrap();
        register_guard.to_slice(buffer).unwrap();

        Ok(())
    }

    fn write_memory(
        &self,
        _address: Address,
        _address_space: AddressSpaceHandle,
        buffer: &[u8],
    ) -> Result<(), MemoryOperationError<WriteMemoryRecord>> {
        let mut register_guard = self.status.lock().unwrap();
        let register = MapctlStatus::from_bytes((buffer, 0)).unwrap().1;
        *register_guard = register;

        Err(MemoryOperationError {
            records: RangeInclusiveMap::default(),
            remap_callback: {
                let config = self.config.clone();
                let my_id = self.my_id;

                Some(RemapCallback::new(move |memory_translation_table| {
                    // remap ram
                    memory_translation_table.remap_memory(
                        config.ram,
                        config.cpu_address_space,
                        [0x0000..=0xffff],
                    );

                    // optionally remap the other stuff on top of it
                    if register.suzy {
                        memory_translation_table.remap_memory(
                            config.suzy,
                            config.cpu_address_space,
                            [SUZY_ADDRESSES],
                        );
                    }

                    if register.mikey {
                        memory_translation_table.remap_memory(
                            config.mikey,
                            config.cpu_address_space,
                            [MIKEY_ADDRESSES],
                        );
                    }

                    if register.vector {
                        memory_translation_table.remap_memory(
                            config.vector,
                            config.cpu_address_space,
                            [VECTOR_ADDRESSES],
                        );
                    }

                    // http://www.monlynx.de/lynx/hardware.html

                    memory_translation_table.remap_memory(
                        config.vector,
                        config.cpu_address_space,
                        [RESERVED_MEMORY_ADDRESS..=RESERVED_MEMORY_ADDRESS],
                    );

                    memory_translation_table.remap_memory(
                        my_id,
                        config.cpu_address_space,
                        [MAPCTL_ADDRESS..=MAPCTL_ADDRESS],
                    );
                }))
            },
        })
    }
}

#[derive(Debug, Clone)]
pub struct MapctlConfig {
    pub ram: ComponentId,
    pub suzy: ComponentId,
    pub mikey: ComponentId,
    pub vector: ComponentId,
    pub reserved: ComponentId,
    pub cpu_address_space: AddressSpaceHandle,
}

impl<P: Platform> ComponentConfig<P> for MapctlConfig {
    type Component = Mapctl;

    fn build_component(
        self,
        component_ref: ComponentRef<Self::Component>,
        component_builder: ComponentBuilder<'_, P, Self::Component>,
        _save: Option<&ComponentSave>,
    ) -> Result<(), BuildError> {
        let component_builder =
            component_builder.map_memory([(self.cpu_address_space, 0xfff9..=0xfff9)]);

        component_builder.build_global(Mapctl {
            config: self,
            status: Default::default(),
            my_id: component_ref.id(),
        });

        Ok(())
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
