use std::sync::Arc;

use bitvec::{field::BitField, prelude::Lsb0, view::BitView};
use multiemu_runtime::{
    component::{Component, ComponentConfig, ComponentPath},
    machine::builder::ComponentBuilder,
    memory::{
        Address, AddressSpace, AddressSpaceId, MapTarget, MemoryError, MemoryRemappingCommand,
        Permissions,
    },
    platform::Platform,
};
use serde::{Deserialize, Serialize};

use crate::{
    MAPCTL_ADDRESS, MIKEY_ADDRESSES, RESERVED_MEMORY_ADDRESS, SUZY_ADDRESSES, VECTOR_ADDRESSES,
};

#[derive(Debug)]
pub struct Mapctl {
    config: MapctlConfig,
    status: MapctlStatus,
    my_path: ComponentPath,
    cpu_address_space: Arc<AddressSpace>,
}

impl Component for Mapctl {
    fn memory_read(
        &self,
        _address: Address,
        _address_space: AddressSpaceId,
        _avoid_side_effects: bool,
        buffer: &mut [u8],
    ) -> Result<(), MemoryError> {
        buffer[0] = self.status.to_byte();

        Ok(())
    }

    fn memory_write(
        &mut self,
        _address: Address,
        _address_space: AddressSpaceId,
        buffer: &[u8],
    ) -> Result<(), MemoryError> {
        self.status = MapctlStatus::from_byte(buffer[0]);

        let mut remapping_commands = Vec::default();

        remapping_commands.push(MemoryRemappingCommand::Map {
            range: 0x0000..=0xffff,
            target: MapTarget::Component(self.config.ram.clone()),
            permissions: Permissions::all(),
        });

        if self.status.suzy {
            remapping_commands.push(MemoryRemappingCommand::Map {
                range: SUZY_ADDRESSES,
                target: MapTarget::Component(self.config.suzy.clone()),
                permissions: Permissions::all(),
            });
        }

        if self.status.mikey {
            remapping_commands.push(MemoryRemappingCommand::Map {
                range: MIKEY_ADDRESSES,
                target: MapTarget::Component(self.config.mikey.clone()),
                permissions: Permissions::all(),
            });
        }

        remapping_commands.push(MemoryRemappingCommand::Map {
            range: RESERVED_MEMORY_ADDRESS..=RESERVED_MEMORY_ADDRESS,
            target: MapTarget::Component(self.config.reserved.clone()),
            permissions: Permissions::all(),
        });

        if self.status.vector {
            remapping_commands.push(MemoryRemappingCommand::Map {
                range: VECTOR_ADDRESSES,
                target: MapTarget::Component(self.config.vector.clone()),
                permissions: Permissions::all(),
            });
        }

        remapping_commands.push(MemoryRemappingCommand::Map {
            range: MAPCTL_ADDRESS..=MAPCTL_ADDRESS,
            target: MapTarget::Component(self.my_path.clone()),
            permissions: Permissions::all(),
        });

        self.cpu_address_space.remap(remapping_commands);

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
    ) -> Result<Self::Component, Box<dyn std::error::Error>> {
        let my_path = component_builder.path().clone();

        let component_builder =
            component_builder.memory_map_component(self.cpu_address_space, 0xfff9..=0xfff9);

        let cpu_address_space = component_builder.address_space(self.cpu_address_space);

        Ok(Mapctl {
            config: self,
            status: Default::default(),
            my_path,
            cpu_address_space,
        })
    }
}

#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize, Default)]
pub struct MapctlStatus {
    pub suzy: bool,
    pub mikey: bool,
    pub rom: bool,
    pub vector: bool,
    pub reserved: u8, // 3 bits used
    pub sequential_disable: bool,
}

impl MapctlStatus {
    /// Load from a single byte (bit 0 = suzy, bit 1 = mikey, etc.)
    pub fn from_byte(byte: u8) -> Self {
        let byte = byte.view_bits::<Lsb0>();

        Self {
            suzy: byte[0],
            mikey: byte[1],
            rom: byte[2],
            vector: byte[3],
            reserved: byte[4..7].load::<u8>(),
            sequential_disable: byte[7],
        }
    }

    /// Convert back into a packed byte
    pub fn to_byte(self) -> u8 {
        let mut byte = 0u8;

        {
            let byte = byte.view_bits_mut::<Lsb0>();

            byte.set(0, self.suzy);
            byte.set(1, self.mikey);
            byte.set(2, self.rom);
            byte.set(3, self.vector);
            byte[4..7].copy_from_bitslice(&self.reserved.view_bits::<Lsb0>()[0..3]);
            byte.set(7, self.sequential_disable);
        }

        byte
    }
}
