use multiemu_machine::{
    builder::ComponentBuilder,
    component::{Component, FromConfig, RuntimeEssentials},
};
use num::rational::Ratio;
use std::{
    num::NonZero,
    sync::{Arc, Mutex},
};

#[derive(Debug)]
pub struct Chip8Timer {
    // The CPU will set this according to what the program wants
    delay_timer: Arc<Mutex<u8>>,
}

impl Chip8Timer {
    pub fn set(&self, value: u8) {
        *self.delay_timer.lock().unwrap() = value;
    }

    pub fn get(&self) -> u8 {
        *self.delay_timer.lock().unwrap()
    }
}

impl Component for Chip8Timer {}

impl FromConfig for Chip8Timer {
    type Config = ();
    type Quirks = ();

    fn from_config(
        component_builder: ComponentBuilder<Self>,
        _essentials: Arc<RuntimeEssentials>,
        _config: Self::Config,
        _quirks: Self::Quirks,
    ) {
        let delay_timer = Arc::new(Mutex::new(0u8));

        component_builder
            .insert_task(Ratio::from_integer(60), {
                let delay_timer = delay_timer.clone();

                move |_: &Self, period: NonZero<u32>| {
                    let mut delay_timer_guard = delay_timer.lock().unwrap();
                    *delay_timer_guard = delay_timer_guard
                        .saturating_sub(period.get().try_into().unwrap_or(u8::MAX));
                }
            })
            .build_global(Self {
                delay_timer: delay_timer.clone(),
            });
    }
}
