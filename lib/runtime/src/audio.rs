use multiemu_audio::SampleFormat;
use nalgebra::SVector;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
pub struct AudioOutputId(pub u16);

pub trait AudioCallback<S: SampleFormat>: Debug + Send + Sync + 'static {
    fn generate_samples(&self) -> Box<dyn Iterator<Item = SVector<S, 2>> + '_>;
}

#[derive(Debug)]
pub struct AudioOutputInfo<S: SampleFormat> {
    pub callback: Box<dyn AudioCallback<S>>,
}
