mod save;
mod snapshot;

pub use save::*;
pub use snapshot::*;

pub const MAGIC: [u8; 8] = *b"multiemu";
