use std::io::{Read, Write};

use multiemu_runtime::{
    component::{Component, ComponentConfig, ComponentVersion},
    machine::builder::{ComponentBuilder, SchedulerParticipation},
    platform::Platform,
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
    ) -> Result<Self::Component, Box<dyn std::error::Error>> {
        component_builder.set_scheduler_participation(SchedulerParticipation::OnDemand);

        Ok(Chip8Timer { timer: 0 })
    }
}
