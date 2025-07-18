use multiemu_runtime::{
    builder::ComponentBuilder,
    component::{BuildError, Component, ComponentConfig, ComponentRef},
    platform::Platform,
};
use multiemu_save::ComponentSave;
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

impl<P: Platform> ComponentConfig<P> for Chip8TimerConfig {
    type Component = Chip8Timer;

    fn build_component(
        self,
        _component_ref: ComponentRef<Self::Component>,
        component_builder: ComponentBuilder<'_, P, Self::Component>,
        _save: Option<&ComponentSave>,
    ) -> Result<(), BuildError> {
        let delay_timer = Arc::new(Mutex::new(0u8));

        component_builder
            .insert_lazy_task(Ratio::from_integer(60), {
                let delay_timer = delay_timer.clone();

                move |time_slice: NonZero<u32>| {
                    let mut delay_timer_guard = delay_timer.lock().unwrap();
                    *delay_timer_guard = delay_timer_guard
                        .saturating_sub(time_slice.get().try_into().unwrap_or(u8::MAX));
                }
            })
            .build_global(Chip8Timer {
                delay_timer: delay_timer.clone(),
            });

        Ok(())
    }
}
