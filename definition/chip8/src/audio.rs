use multiemu_audio::{SampleFormat, SquareWave};
use multiemu_runtime::{
    audio::AudioCallback,
    builder::ComponentBuilder,
    component::{BuildError, Component, ComponentConfig, ComponentRef},
    platform::Platform,
};
use multiemu_save::ComponentSave;
use nalgebra::SVector;
use num::{FromPrimitive, Zero, rational::Ratio};
use std::{
    num::NonZero,
    sync::{Arc, Mutex, RwLock},
};

#[derive(Debug)]
pub struct Chip8Audio {
    // The CPU will set this according to what the program wants
    sound_timer: Arc<RwLock<u8>>,
}

impl Chip8Audio {
    pub fn set(&self, value: u8) {
        *self.sound_timer.write().unwrap() = value;
    }
}

impl Component for Chip8Audio {}

#[derive(Debug, Default)]
pub struct Chip8AudioConfig;

impl<P: Platform> ComponentConfig<P> for Chip8AudioConfig {
    type Component = Chip8Audio;

    fn build_component(
        self,
        _component_ref: ComponentRef<Self::Component>,
        component_builder: ComponentBuilder<'_, P, Self::Component>,
        _save: Option<ComponentSave>,
    ) -> Result<(), BuildError> {
        let sound_timer = Arc::new(RwLock::new(0u8));
        let sample_rate = component_builder.essentials().sample_rate;

        component_builder
            .insert_task(Ratio::from_integer(60), {
                let sound_timer = sound_timer.clone();

                move |time_slice: NonZero<u32>| {
                    let mut sound_timer_guard = sound_timer.write().unwrap();
                    *sound_timer_guard = sound_timer_guard
                        .saturating_sub(time_slice.get().try_into().unwrap_or(u8::MAX));
                }
            })
            .insert_audio_output(Chip8AudioDataCallback {
                sound_timer: sound_timer.clone(),
                square_wave: SquareWave::new(
                    Ratio::from_integer(440),
                    sample_rate,
                    // TODO: Configurable?
                    P::SampleFormat::max_sample() / P::SampleFormat::from_usize(10).unwrap(),
                )
                .into(),
            })
            .0
            .build_global(Chip8Audio {
                sound_timer: sound_timer.clone(),
            });

        Ok(())
    }
}

#[derive(Debug)]
pub struct Chip8AudioDataCallback<S: SampleFormat> {
    sound_timer: Arc<RwLock<u8>>,
    square_wave: Mutex<SquareWave<S, 2>>,
}

impl<S: SampleFormat> AudioCallback<S> for Chip8AudioDataCallback<S> {
    fn generate_samples(&self) -> Box<dyn Iterator<Item = nalgebra::SVector<S, 2>> + '_> {
        let sound_timer_guard = self.sound_timer.read().unwrap();
        let mut square_wave_guard = self.square_wave.lock().unwrap();

        Box::new(std::iter::from_fn(move || {
            if sound_timer_guard.is_zero() {
                Some(SVector::<S, 2>::from_element(S::equilibrium()))
            } else {
                square_wave_guard.next()
            }
        }))
    }
}
