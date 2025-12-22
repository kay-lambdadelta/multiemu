use std::ops::RangeInclusive;

use fluxemu_range::ContiguousRange;
use fluxemu_runtime::{
    component::{Component, ComponentConfig},
    machine::builder::ComponentBuilder,
    memory::{Address, AddressSpaceId, MemoryError},
    platform::Platform,
};

use crate::apu::pulse::PulseChannel;

mod pulse;

const PULSE_1: RangeInclusive<Address> = 0x4000..=0x4003;
const PULSE_2: RangeInclusive<Address> = 0x4004..=0x4007;
const TRIANGLE: RangeInclusive<Address> = 0x4008..=0x400b;
const NOISE: RangeInclusive<Address> = 0x400c..=0x400f;
const DMC: RangeInclusive<Address> = 0x4010..=0x4013;
const CONTROL: Address = 0x4015;
const STATUS: Address = 0x4015;
const FRAME_COUNTER: Address = 0x4017;

#[derive(Debug)]
pub struct Apu {
    pub pulse_channels: [PulseChannel; 2],
}

impl Component for Apu {
    fn memory_read(
        &self,
        address: Address,
        address_space: AddressSpaceId,
        avoid_side_effects: bool,
        buffer: &mut [u8],
    ) -> Result<(), MemoryError> {
        match address {
            STATUS => {
                todo!()
            }
            _ => {
                unreachable!()
            }
        }
    }

    fn memory_write(
        &mut self,
        address: Address,
        address_space: AddressSpaceId,
        buffer: &[u8],
    ) -> Result<(), MemoryError> {
        for (address, byte) in
            RangeInclusive::from_start_and_length(address, buffer.len()).zip(buffer.iter().copied())
        {
            if PULSE_1.contains(&address) {
                self.pulse_write(0, address - PULSE_1.start(), byte);
            }

            if PULSE_2.contains(&address) {
                self.pulse_write(1, address - PULSE_2.start(), byte);
            }

            if CONTROL == address {
                self.pulse_channels[1].enabled = (byte & 0b0000_0010) != 0;
                self.pulse_channels[0].enabled = (byte & 0b0000_0001) != 0;
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct ApuConfig {
    pub cpu_address_space: AddressSpaceId,
}

impl<P: Platform> ComponentConfig<P> for ApuConfig {
    type Component = Apu;

    fn build_component(
        self,
        component_builder: ComponentBuilder<P, Self::Component>,
    ) -> Result<Self::Component, Box<dyn std::error::Error>> {
        component_builder
            .memory_map_component_write(self.cpu_address_space, PULSE_1)
            .memory_map_component_write(self.cpu_address_space, PULSE_2)
            .memory_map_component_write(self.cpu_address_space, TRIANGLE)
            .memory_map_component_write(self.cpu_address_space, NOISE)
            .memory_map_component_write(self.cpu_address_space, DMC)
            .memory_map_component_write(self.cpu_address_space, CONTROL..=CONTROL)
            .memory_map_component_read(self.cpu_address_space, STATUS..=STATUS)
            .memory_map_component_write(self.cpu_address_space, FRAME_COUNTER..=FRAME_COUNTER);

        Ok(Apu {
            pulse_channels: Default::default(),
        })
    }
}
