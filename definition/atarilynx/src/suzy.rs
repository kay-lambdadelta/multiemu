use multiemu_runtime::{
    component::{Component, ComponentConfig},
    machine::builder::ComponentBuilder,
    memory::{
        Address, AddressSpaceId, ReadMemoryError, ReadMemoryErrorType, WriteMemoryError,
        WriteMemoryErrorType,
    },
    platform::Platform,
};
use std::ops::RangeInclusive;

use crate::SUZY_ADDRESSES;

const TMPADRL: RangeInclusive<Address> = 0xfc00..=0xfc01;
const TILTACUM: RangeInclusive<Address> = 0xfc02..=0xfc03;
const HOFF: RangeInclusive<Address> = 0xfc04..=0xfc05;
const VOFF: RangeInclusive<Address> = 0xfc06..=0xfc07;
const VIDBAS: RangeInclusive<Address> = 0xfc08..=0xfc09;
const COLLBAS: RangeInclusive<Address> = 0xfc0a..=0xfc0b;
const VIDADRL: RangeInclusive<Address> = 0xfc0c..=0xfc0d;
const COLLARL: RangeInclusive<Address> = 0xfc0e..=0xfc0f;
const SCBNEXT: RangeInclusive<Address> = 0xfc10..=0xfc11;
const SPRDLINE: RangeInclusive<Address> = 0xfc12..=0xfc13;
const HPOSSTRT: RangeInclusive<Address> = 0xfc14..=0xfc15;
const VPOSSTRT: RangeInclusive<Address> = 0xfc16..=0xfc17;
const SPRHSIZ: RangeInclusive<Address> = 0xfc18..=0xfc19;
const SPRVSIZ: RangeInclusive<Address> = 0xfc1a..=0xfc1b;
const STRETCH: RangeInclusive<Address> = 0xfc1c..=0xfc1d;
const TILT: RangeInclusive<Address> = 0xfc1e..=0xfc1f;
const SPRDOFF: RangeInclusive<Address> = 0xfc20..=0xfc21;
const SPRVPOS: RangeInclusive<Address> = 0xfc22..=0xfc23;
const COLLOFF: RangeInclusive<Address> = 0xfc24..=0xfc25;
const VSIZACUM: RangeInclusive<Address> = 0xfc26..=0xfc27;
const HSIZOFF: RangeInclusive<Address> = 0xfc28..=0xfc29;
const VSIZOFF: RangeInclusive<Address> = 0xfc2a..=0xfc2b;
const SCBADR: RangeInclusive<Address> = 0xfc2c..=0xfc2d;
const PROCADR: RangeInclusive<Address> = 0xfc2e..=0xfc2f;
const SPRCTRL0: RangeInclusive<Address> = 0xfc80..=0xfc80;
const SPRCTRL1: RangeInclusive<Address> = 0xfc81..=0xfc81;
const SPRCOLL: RangeInclusive<Address> = 0xfc82..=0xfc82;
const SPRINT: RangeInclusive<Address> = 0xfc83..=0xfc83;
const SUZYHREV_R: RangeInclusive<Address> = 0xfc88..=0xfc88;
const SUZYHREV_W: RangeInclusive<Address> = 0xfc89..=0xfc89;
const SUZYBUSEN: RangeInclusive<Address> = 0xfc90..=0xfc90;
const SPRGO: RangeInclusive<Address> = 0xfc91..=0xfc91;
const SPRSYS: RangeInclusive<Address> = 0xfc92..=0xfc92;
const JOYSTICK: RangeInclusive<Address> = 0xfcb0..=0xfcb0;
const SWITCHES: RangeInclusive<Address> = 0xfcb1..=0xfcb1;
const RCART: RangeInclusive<Address> = 0xfcb2..=0xfcb3;
const LEDS: RangeInclusive<Address> = 0xfcc0..=0xfcc0;
const PPT: RangeInclusive<Address> = 0xfcc2..=0xfcc2;
const PPTDATA: RangeInclusive<Address> = 0xfcc3..=0xfcc3;
const HOWIE: RangeInclusive<Address> = 0xfcc4..=0xfcc4;

#[derive(Debug)]
pub struct Suzy {}

#[allow(clippy::if_same_then_else)]
impl Component for Suzy {
    fn read_memory(
        &self,
        address: Address,
        _address_space: AddressSpaceId,
        _avoid_side_effects: bool,
        buffer: &mut [u8],
    ) -> Result<(), ReadMemoryError> {
        // TODO: Make this a match table when rust gets inline const patterns

        if TMPADRL.contains(&address) {
        } else if TILTACUM.contains(&address) {
        } else if HOFF.contains(&address) {
        } else if VOFF.contains(&address) {
        } else if VIDBAS.contains(&address) {
        } else if COLLBAS.contains(&address) {
        } else if VIDADRL.contains(&address) {
        } else if COLLARL.contains(&address) {
        } else if SCBNEXT.contains(&address) {
        } else if SPRDLINE.contains(&address) {
        } else if HPOSSTRT.contains(&address) {
        } else if VPOSSTRT.contains(&address) {
        } else if SPRHSIZ.contains(&address) {
        } else if SPRVSIZ.contains(&address) {
        } else if STRETCH.contains(&address) {
        } else if TILT.contains(&address) {
        } else if SPRDOFF.contains(&address) {
        } else if SPRVPOS.contains(&address) {
        } else if COLLOFF.contains(&address) {
        } else if VSIZACUM.contains(&address) {
        } else if HSIZOFF.contains(&address) {
        } else if VSIZOFF.contains(&address) {
        } else if SCBADR.contains(&address) {
        } else if PROCADR.contains(&address) {
        } else if SPRCTRL0.contains(&address) {
        } else if SPRCTRL1.contains(&address) {
        } else if SPRCOLL.contains(&address) {
        } else if SPRINT.contains(&address) {
        } else if SUZYHREV_R.contains(&address) {
        } else if SUZYHREV_W.contains(&address) {
        } else if SUZYBUSEN.contains(&address) {
        } else if SPRGO.contains(&address) {
        } else if SPRSYS.contains(&address) {
        } else if JOYSTICK.contains(&address) {
        } else if SWITCHES.contains(&address) {
        } else if RCART.contains(&address) {
        } else if LEDS.contains(&address) {
        } else if PPT.contains(&address) {
        } else if PPTDATA.contains(&address) {
        } else if HOWIE.contains(&address) {
        } else {
            return Err(ReadMemoryError(
                std::iter::once((
                    address..=(address + (buffer.len() - 1)),
                    ReadMemoryErrorType::Denied,
                ))
                .collect(),
            ));
        }

        Ok(())
    }

    fn write_memory(
        &mut self,
        address: Address,
        _address_space: AddressSpaceId,
        buffer: &[u8],
    ) -> Result<(), WriteMemoryError> {
        if TMPADRL.contains(&address) {
        } else if TILTACUM.contains(&address) {
        } else if HOFF.contains(&address) {
        } else if VOFF.contains(&address) {
        } else if VIDBAS.contains(&address) {
        } else if COLLBAS.contains(&address) {
        } else if VIDADRL.contains(&address) {
        } else if COLLARL.contains(&address) {
        } else if SCBNEXT.contains(&address) {
        } else if SPRDLINE.contains(&address) {
        } else if HPOSSTRT.contains(&address) {
        } else if VPOSSTRT.contains(&address) {
        } else if SPRHSIZ.contains(&address) {
        } else if SPRVSIZ.contains(&address) {
        } else if STRETCH.contains(&address) {
        } else if TILT.contains(&address) {
        } else if SPRDOFF.contains(&address) {
        } else if SPRVPOS.contains(&address) {
        } else if COLLOFF.contains(&address) {
        } else if VSIZACUM.contains(&address) {
        } else if HSIZOFF.contains(&address) {
        } else if VSIZOFF.contains(&address) {
        } else if SCBADR.contains(&address) {
        } else if PROCADR.contains(&address) {
        } else if SPRCTRL0.contains(&address) {
        } else if SPRCTRL1.contains(&address) {
        } else if SPRCOLL.contains(&address) {
        } else if SPRINT.contains(&address) {
        } else if SUZYHREV_R.contains(&address) {
        } else if SUZYHREV_W.contains(&address) {
        } else if SUZYBUSEN.contains(&address) {
        } else if SPRGO.contains(&address) {
        } else if SPRSYS.contains(&address) {
        } else if JOYSTICK.contains(&address) {
        } else if SWITCHES.contains(&address) {
        } else if RCART.contains(&address) {
        } else if LEDS.contains(&address) {
        } else if PPT.contains(&address) {
        } else if PPTDATA.contains(&address) {
        } else if HOWIE.contains(&address) {
        } else {
            return Err(WriteMemoryError(
                std::iter::once((
                    address..=(address + (buffer.len() - 1)),
                    WriteMemoryErrorType::Denied,
                ))
                .collect(),
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SuzyConfig {
    pub cpu_address_space: AddressSpaceId,
}

impl<P: Platform> ComponentConfig<P> for SuzyConfig {
    type Component = Suzy;

    fn build_component(
        self,
        component_builder: ComponentBuilder<P, Self::Component>,
    ) -> Result<Self::Component, Box<dyn std::error::Error>> {
        component_builder.memory_map(SUZY_ADDRESSES, self.cpu_address_space);

        Ok(Suzy {})
    }
}
