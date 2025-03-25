use multiemu_machine::builder::ComponentBuilder;
use multiemu_machine::component::{Component, FromConfig, RuntimeEssentials};
use num::rational::Ratio;
use std::num::NonZero;
use std::sync::{Arc, Mutex};

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

impl FromConfig for Chip8Audio {
    type Config = ();
    type Quirks = ();

    fn from_config(
        component_builder: ComponentBuilder<Self>,
        _essentials: Arc<RuntimeEssentials>,
        _config: Self::Config,
        _quirks: Self::Quirks,
    ) {
        let sound_timer = Arc::new(Mutex::new(0u8));

        component_builder
            .insert_task(Ratio::from_integer(60), {
                let sound_timer = sound_timer.clone();

                move |_: &Self, period: NonZero<u32>| {
                    let mut sound_timer_guard = sound_timer.lock().unwrap();
                    *sound_timer_guard = sound_timer_guard
                        .saturating_sub(period.get().try_into().unwrap_or(u8::MAX));
                }
            })
            .build_global(Self {
                sound_timer: sound_timer.clone(),
            });
    }
}
