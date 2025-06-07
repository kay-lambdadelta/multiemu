use crate::audio::AudioCallback;
use multiemu_audio::Sample;

pub struct AudioMetadata<S: Sample> {
    pub audio_data_callbacks: Vec<Box<dyn AudioCallback<S>>>,
}

impl<S: Sample> Default for AudioMetadata<S> {
    fn default() -> Self {
        Self {
            audio_data_callbacks: Vec::new(),
        }
    }
}
