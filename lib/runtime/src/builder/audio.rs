use crate::audio::{AudioOutputId, AudioOutputInfo};
use multiemu_audio::SampleFormat;
use std::collections::HashMap;

pub struct AudioMetadata<S: SampleFormat> {
    pub audio_outputs: HashMap<AudioOutputId, AudioOutputInfo<S>>,
}

impl<S: SampleFormat> Default for AudioMetadata<S> {
    fn default() -> Self {
        Self {
            audio_outputs: Default::default(),
        }
    }
}
