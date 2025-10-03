use crate::{
    MAPCTL_ADDRESS, MIKEY_ADDRESSES, RESERVED_MEMORY_ADDRESS, SUZY_ADDRESSES, VECTOR_ADDRESSES,
};
use deku::{DekuContainerRead, DekuContainerWrite, DekuRead, DekuWrite};
use multiemu::{
    component::{BuildError, Component, ComponentConfig, ComponentPath},
    machine::builder::ComponentBuilder,
    memory::{
        Address, AddressSpaceId, MemoryAccessTable, MemoryOperationError, MemoryRemappingCommands,
        ReadMemoryRecord, WriteMemoryRecord,
    },
    platform::Platform,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug)]
pub struct Mapctl {
    config: MapctlConfig,
    status: MapctlStatus,
    my_path: ComponentPath,
    mat: Arc<MemoryAccessTable>,
}

impl Component for Mapctl {
    fn read_memory(
        &self,
        _address: Address,
        _address_space: AddressSpaceId,
        buffer: &mut [u8],
    ) -> Result<(), MemoryOperationError<ReadMemoryRecord>> {
        self.status.to_slice(buffer).unwrap();

        Ok(())
    }

    fn write_memory(
        &mut self,
        _address: Address,
        _address_space: AddressSpaceId,
        buffer: &[u8],
    ) -> Result<(), MemoryOperationError<WriteMemoryRecord>> {
        self.status = MapctlStatus::from_bytes((buffer, 0)).unwrap().1;

        let mut remapping_commands = Vec::default();

        remapping_commands.push(MemoryRemappingCommands::MapComponent {
            range: 0x0000..=0xffff,
            path: self.config.ram.clone(),
        });

        if self.status.suzy {
            remapping_commands.push(MemoryRemappingCommands::MapComponent {
                range: SUZY_ADDRESSES,
                path: self.config.suzy.clone(),
            });
        }

        if self.status.mikey {
            remapping_commands.push(MemoryRemappingCommands::MapComponent {
                range: MIKEY_ADDRESSES,
                path: self.config.mikey.clone(),
            });
        }

        remapping_commands.push(MemoryRemappingCommands::MapComponent {
            range: RESERVED_MEMORY_ADDRESS..=RESERVED_MEMORY_ADDRESS,
            path: self.config.reserved.clone(),
        });

        if self.status.vector {
            remapping_commands.push(MemoryRemappingCommands::MapComponent {
                range: VECTOR_ADDRESSES,
                path: self.config.vector.clone(),
            });
        }

        remapping_commands.push(MemoryRemappingCommands::MapComponent {
            range: MAPCTL_ADDRESS..=MAPCTL_ADDRESS,
            path: self.my_path.clone(),
        });

        self.mat
            .remap(self.config.cpu_address_space, remapping_commands);

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct MapctlConfig {
    pub ram: ComponentPath,
    pub suzy: ComponentPath,
    pub mikey: ComponentPath,
    pub vector: ComponentPath,
    pub reserved: ComponentPath,
    pub cpu_address_space: AddressSpaceId,
}

impl<P: Platform> ComponentConfig<P> for MapctlConfig {
    type Component = Mapctl;

    fn build_component(
        self,
        component_builder: ComponentBuilder<'_, P, Self::Component>,
    ) -> Result<(), BuildError> {
        let my_path = component_builder.path().clone();

        let component_builder =
            component_builder.memory_map(self.cpu_address_space, 0xfff9..=0xfff9);

        let mat = component_builder.memory_access_table();

        component_builder.build(Mapctl {
            config: self,
            status: Default::default(),
            my_path,
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
