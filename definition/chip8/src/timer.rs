use multiemu_runtime::{
    builder::ComponentBuilder,
    component::{Component, ComponentConfig},
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

#[derive(Debug, Default)]
pub struct Chip8TimerConfig;

impl<B: ComponentBuilder<Component = Chip8Timer>> ComponentConfig<B> for Chip8TimerConfig {
    type Component = Chip8Timer;

    fn build_component(self, component_builder: B) -> B::BuildOutput {
        let delay_timer = Arc::new(Mutex::new(0u8));

        component_builder
            .insert_task(Ratio::from_integer(60), {
                let delay_timer = delay_timer.clone();

                move |_: &Self::Component, period: NonZero<u32>| {
                    let mut delay_timer_guard = delay_timer.lock().unwrap();
                    *delay_timer_guard = delay_timer_guard
                        .saturating_sub(period.get().try_into().unwrap_or(u8::MAX));
                }
            })
            .build_global(Chip8Timer {
                delay_timer: delay_timer.clone(),
            })
    }
}
