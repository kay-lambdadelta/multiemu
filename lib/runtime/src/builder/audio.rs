use crate::audio::AudioDataCallback;
use multiemu_audio::Sample;

pub struct AudioMetadata<S: Sample> {
    pub audio_data_callbacks: Vec<Box<dyn AudioDataCallback<S>>>,
}

impl<S: Sample> Default for AudioMetadata<S> {
    fn default() -> Self {
        Self {
            audio_data_callbacks: Vec::new(),
        }
    }
}
