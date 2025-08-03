use crate::cartridge::mapper::Mapper;

pub struct Mapper000 {}

impl Mapper for Mapper000 {
    fn read(&mut self, address: u16) -> u8 {
        todo!()
    }

    fn write(&mut self, address: u16, value: u8) {
        todo!()
    }
}
