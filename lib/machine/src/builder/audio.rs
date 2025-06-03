use super::ComponentBuilder;
use crate::{audio::AudioQueue, component::Component, display::backend::RenderApi};
use num::rational::Ratio;
use std::{collections::HashMap, sync::Arc};

#[derive(Default)]
pub struct AudioMetadata {
    pub audio_queues: HashMap<&'static str, Arc<AudioQueue>>,
}

impl<R: RenderApi, C: Component> ComponentBuilder<'_, R, C> {
    pub fn create_audio_queue(
        mut self,
        name: &'static str,
        sample_rate: Ratio<u32>,
    ) -> (Self, Arc<AudioQueue>) {
        let audio_queue = Arc::new(AudioQueue::new(sample_rate));
        self.component_metadata
            .audio
            .get_or_insert_default()
            .audio_queues
            .insert(name, audio_queue.clone());

        (self, audio_queue)
    }
}
