use multiemu_runtime::{
    builder::ComponentBuilder,
    component::{BuildError, Component, ComponentConfig, ComponentVersion},
    platform::Platform,
};
use num::rational::Ratio;
use std::{
    io::{Read, Write},
    num::NonZero,
};

#[derive(Debug)]
pub struct Chip8Timer {
    // The CPU will set this according to what the program wants
    timer: u8,
}

impl Chip8Timer {
    pub fn set(&mut self, value: u8) {
        self.timer = value;
    }

    pub fn get(&self) -> u8 {
        self.timer
    }
}

impl Component for Chip8Timer {
    fn snapshot_version(&self) -> Option<ComponentVersion> {
        Some(0)
    }

    fn load_snapshot(
        &mut self,
        version: ComponentVersion,
        mut reader: Box<dyn Read>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(version, 0);
        let timer = std::array::from_mut(&mut self.timer);

        reader.read_exact(timer)?;

        Ok(())
    }

    fn store_snapshot(&self, mut writer: Box<dyn Write>) -> Result<(), Box<dyn std::error::Error>> {
        let timer = std::array::from_ref(&self.timer);

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
        let component = component_builder.component_ref();

        component_builder
            .insert_task(
                Ratio::from_integer(60),
                "driver",
                move |slice: NonZero<u32>| {
                    component
                        .interact_mut(|component| {
                            component.timer = component
                                .timer
                                .saturating_sub(slice.get().try_into().unwrap_or(u8::MAX));
                        })
                        .unwrap();
                },
            )
            .build(Chip8Timer { timer: 0 });

        Ok(())
    }
}
