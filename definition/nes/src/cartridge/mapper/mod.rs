pub mod mapper_000;

pub trait Mapper: Send + Sync {
    fn read(&self, address: u16) -> u8;
    fn write(&mut self, address: u16, value: u8);
}
