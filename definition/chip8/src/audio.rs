use multiemu_machine::{
    builder::ComponentBuilder,
    component::{Component, ComponentConfig},
    display::backend::RenderApi,
};
use num::rational::Ratio;
use std::{
    num::NonZero,
    sync::{Arc, Mutex},
};

#[derive(Debug)]
pub struct Chip8Audio {
    // The CPU will set this according to what the program wants
    sound_timer: Arc<Mutex<u8>>,
}

impl Chip8Audio {
    pub fn set(&self, value: u8) {
        *self.sound_timer.lock().unwrap() = value;
    }
}

impl Component for Chip8Audio {}

#[derive(Debug, Default)]
pub struct Chip8AudioConfig;

impl<R: RenderApi> ComponentConfig<R> for Chip8AudioConfig {
    type Component = Chip8Audio;

    fn build_component(self, component_builder: ComponentBuilder<R, Self::Component>) {
        let sound_timer = Arc::new(Mutex::new(0u8));

        component_builder
            .insert_task(Ratio::from_integer(60), {
                let sound_timer = sound_timer.clone();

                move |_: &Self::Component, period: NonZero<u32>| {
                    let mut sound_timer_guard = sound_timer.lock().unwrap();
                    *sound_timer_guard = sound_timer_guard
                        .saturating_sub(period.get().try_into().unwrap_or(u8::MAX));
                }
            })
            .build_global(Chip8Audio {
                sound_timer: sound_timer.clone(),
            });
    }
}
