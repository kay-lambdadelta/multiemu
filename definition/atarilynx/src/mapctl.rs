use crate::{
    MAPCTL_ADDRESS, MIKEY_ADDRESSES, RESERVED_MEMORY_ADDRESS, SUZY_ADDRESSES, VECTOR_ADDRESSES,
};
use deku::{DekuContainerRead, DekuContainerWrite, DekuRead, DekuWrite};
use multiemu_runtime::{
    builder::ComponentBuilder,
    component::{BuildError, Component, ComponentConfig, ComponentId},
    memory::{
        Address, AddressSpaceHandle, MemoryAccessTable, MemoryOperationError,
        MemoryRemappingCommands, MemoryType, ReadMemoryRecord, WriteMemoryRecord,
    },
    platform::Platform,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub struct Mapctl {
    config: MapctlConfig,
    status: Mutex<MapctlStatus>,
    my_id: ComponentId,
    mat: Arc<MemoryAccessTable>,
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

        let mut remapping_commands = Vec::default();

        remapping_commands.push(MemoryRemappingCommands::Add {
            range: 0x0000..=0xffff,
            component_id: self.config.ram,
            types: vec![MemoryType::Read, MemoryType::Write],
        });

        if register.suzy {
            remapping_commands.push(MemoryRemappingCommands::Add {
                range: SUZY_ADDRESSES,
                component_id: self.config.suzy,
                types: vec![MemoryType::Read, MemoryType::Write],
            });
        }

        if register.mikey {
            remapping_commands.push(MemoryRemappingCommands::Add {
                range: MIKEY_ADDRESSES,
                component_id: self.config.mikey,
                types: vec![MemoryType::Read, MemoryType::Write],
            });
        }

        remapping_commands.push(MemoryRemappingCommands::Add {
            range: RESERVED_MEMORY_ADDRESS..=RESERVED_MEMORY_ADDRESS,
            component_id: self.config.reserved,
            types: vec![MemoryType::Read, MemoryType::Write],
        });

        if register.vector {
            remapping_commands.push(MemoryRemappingCommands::Add {
                range: VECTOR_ADDRESSES,
                component_id: self.config.vector,
                types: vec![MemoryType::Read, MemoryType::Write],
            });
        }

        remapping_commands.push(MemoryRemappingCommands::Add {
            range: MAPCTL_ADDRESS..=MAPCTL_ADDRESS,
            component_id: self.my_id,
            types: vec![MemoryType::Read, MemoryType::Write],
        });

        self.mat
            .remap(self.config.cpu_address_space, remapping_commands);

        Ok(())
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
        component_builder: ComponentBuilder<'_, P, Self::Component>,
    ) -> Result<(), BuildError> {
        let component_id = component_builder.component_ref().id();

        let component_builder =
            component_builder.memory_map(self.cpu_address_space, 0xfff9..=0xfff9);

        let mat = component_builder.memory_access_table();

        component_builder.build(Mapctl {
            config: self,
            status: Default::default(),
            my_id: component_id,
            mat,
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
