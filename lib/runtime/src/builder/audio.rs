use crate::audio::AudioOutputId;
use std::collections::HashSet;

#[derive(Debug, Default)]
pub struct AudioMetadata {
    pub audio_outputs: HashSet<AudioOutputId>,
}
