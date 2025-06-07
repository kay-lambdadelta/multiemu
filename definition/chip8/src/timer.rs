use multiemu_runtime::{
    builder::ComponentBuilder,
    component::{Component, ComponentConfig, component_ref::ComponentRef},
    scheduler::{SchedulerHandle, YieldReason},
};
use num::rational::Ratio;
use std::sync::{Arc, Mutex};

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

    fn build_component(
        self,
        _component_ref: ComponentRef<Self::Component>,
        component_builder: B,
    ) -> B::BuildOutput {
        let delay_timer = Arc::new(Mutex::new(0u8));

        component_builder
            .insert_task(Ratio::from_integer(60), {
                let delay_timer = delay_timer.clone();

                move |mut handle: SchedulerHandle| {
                    let mut should_exit = false;

                    while !should_exit {
                        let mut delay_timer_guard = delay_timer.lock().unwrap();
                        *delay_timer_guard = delay_timer_guard.saturating_sub(1);
                        drop(delay_timer_guard);

                        handle.tick(|reason| {
                            if reason == YieldReason::Exit {
                                should_exit = true
                            }
                        });
                    }
                }
            })
            .build_global(Chip8Timer {
                delay_timer: delay_timer.clone(),
            })
    }
}
