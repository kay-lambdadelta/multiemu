use multiemu_runtime::{
    builder::ComponentBuilder,
    component::{BuildError, Component, ComponentConfig, ComponentVersion},
    platform::Platform,
};
use num::rational::Ratio;
use std::{
    io::{Read, Write},
    num::NonZero,
    ops::{Deref, DerefMut},
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

impl Component for Chip8Timer {
    fn snapshot_version(&self) -> Option<ComponentVersion> {
        Some(0)
    }

    fn load_snapshot(
        &self,
        version: ComponentVersion,
        mut reader: Box<dyn Read>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(version, 0);
        let mut timer_guard = self.delay_timer.lock().unwrap();
        let timer = std::array::from_mut(timer_guard.deref_mut());

        reader.read_exact(timer)?;

        Ok(())
    }

    fn store_snapshot(&self, mut writer: Box<dyn Write>) -> Result<(), Box<dyn std::error::Error>> {
        let timer_guard = self.delay_timer.lock().unwrap();
        let timer = std::array::from_ref(timer_guard.deref());

        writer.write_all(timer)?;

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct Chip8TimerConfig;

impl<P: Platform> ComponentConfig<P> for Chip8TimerConfig {
    type Component = Chip8Timer;

    fn build_component(
        self,
        component_builder: ComponentBuilder<'_, P, Self::Component>,
    ) -> Result<(), BuildError> {
        let delay_timer = Arc::new(Mutex::new(0u8));

        component_builder
            .insert_lazy_task(Ratio::from_integer(60), "driver", {
                let delay_timer = delay_timer.clone();

                move |time_slice: NonZero<u32>| {
                    let mut delay_timer_guard = delay_timer.lock().unwrap();
                    *delay_timer_guard = delay_timer_guard
                        .saturating_sub(time_slice.get().try_into().unwrap_or(u8::MAX));
                }
            })
            .build_global(move |_| Chip8Timer { delay_timer });

        Ok(())
    }
}
