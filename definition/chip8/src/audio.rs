use multiemu_audio::{SampleFormat, SquareWave};
use multiemu_runtime::{
    audio::AudioCallback,
    builder::ComponentBuilder,
    component::{BuildError, Component, ComponentConfig, ComponentRef, ComponentVersion},
    platform::Platform,
};
use nalgebra::SVector;
use num::{FromPrimitive, Zero, rational::Ratio};
use std::{
    io::{Read, Write},
    num::NonZero,
    sync::Mutex,
};

#[derive(Debug)]
pub struct Chip8Audio {
    // The CPU will set this according to what the program wants
    sound_timer: u8,
}

impl Chip8Audio {
    pub fn set(&mut self, value: u8) {
        self.sound_timer = value;
    }
}

impl Component for Chip8Audio {
    fn snapshot_version(&self) -> Option<ComponentVersion> {
        Some(0)
    }

    fn load_snapshot(
        &mut self,
        version: ComponentVersion,
        mut reader: Box<dyn Read>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(version, 0);
        let timer = std::array::from_mut(&mut self.sound_timer);

        reader.read_exact(timer)?;

        Ok(())
    }

    fn store_snapshot(&self, mut writer: Box<dyn Write>) -> Result<(), Box<dyn std::error::Error>> {
        let timer = std::array::from_ref(&self.sound_timer);

        writer.write_all(timer)?;

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct Chip8AudioConfig;

impl<P: Platform> ComponentConfig<P> for Chip8AudioConfig {
    type Component = Chip8Audio;

    fn build_component(
        self,
        component_builder: ComponentBuilder<'_, P, Self::Component>,
    ) -> Result<(), BuildError> {
        let component = component_builder.component_ref();
        let sample_rate = component_builder.sample_rate();

        component_builder
            .insert_audio_output(Chip8AudioDataCallback {
                square_wave: SquareWave::new(
                    Ratio::from_integer(440),
                    sample_rate,
                    // TODO: Configurable?
                    P::SampleFormat::max_sample() / P::SampleFormat::from_usize(10).unwrap(),
                )
                .into(),
                component: component.clone(),
            })
            .0
            .insert_task(
                Ratio::from_integer(60),
                "driver",
                move |slice: NonZero<u32>| {
                    component
                        .interact_mut(|component| {
                            component.sound_timer = component
                                .sound_timer
                                .saturating_sub(slice.get().try_into().unwrap_or(u8::MAX));
                        })
                        .unwrap();
                },
            )
            .build(Chip8Audio { sound_timer: 0 });

        Ok(())
    }
}

#[derive(Debug)]
pub struct Chip8AudioDataCallback<S: SampleFormat> {
    square_wave: Mutex<SquareWave<S, 2>>,
    component: ComponentRef<Chip8Audio>,
}

impl<S: SampleFormat> AudioCallback<S> for Chip8AudioDataCallback<S> {
    fn generate_samples(&self) -> Box<dyn Iterator<Item = nalgebra::SVector<S, 2>> + '_> {
        let mut square_wave_guard = self.square_wave.lock().unwrap();

        let timer_value = self
            .component
            .interact_local(|component| component.sound_timer)
            .unwrap();

        Box::new(std::iter::from_fn(move || {
            if timer_value.is_zero() {
                Some(SVector::<S, 2>::from_element(S::equilibrium()))
            } else {
                square_wave_guard.next()
            }
        }))
    }
}
