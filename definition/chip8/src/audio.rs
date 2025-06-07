use multiemu_audio::{FromSample, Sample, SquareWave};
use multiemu_runtime::{
    audio::AudioCallback,
    builder::ComponentBuilder,
    component::{Component, ComponentConfig, component_ref::ComponentRef},
    scheduler::{SchedulerHandle, YieldReason},
};
use nalgebra::SVector;
use num::{FromPrimitive, Zero, rational::Ratio};
use std::sync::{Arc, Mutex, RwLock};

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

impl<B: ComponentBuilder<Component = Chip8Audio>> ComponentConfig<B> for Chip8AudioConfig
where
    B::SampleFormat: FromSample<f32>,
{
    type Component = Chip8Audio;

    fn build_component(
        self,
        _component_ref: ComponentRef<Self::Component>,
        component_builder: B,
    ) -> B::BuildOutput {
        let sound_timer = Arc::new(RwLock::new(0u8));
        let essentials = component_builder.essentials();

        component_builder
            .insert_task(Ratio::from_integer(60), {
                let sound_timer = sound_timer.clone();

                move |mut handle: SchedulerHandle| {
                    let mut should_exit = false;

                    while !should_exit {
                        let mut sound_timer_guard = sound_timer.write().unwrap();
                        *sound_timer_guard = sound_timer_guard.saturating_sub(1);
                        drop(sound_timer_guard);

                        handle.tick(|reason| {
                            if reason == YieldReason::Exit {
                                should_exit = true
                            }
                        });
                    }
                }
            })
            .insert_audio_callback(Chip8AudioDataCallback {
                sound_timer: sound_timer.clone(),
                square_wave: SquareWave::new(
                    Ratio::from_integer(440),
                    essentials.sample_rate,
                    // TODO: Configurable?
                    B::SampleFormat::max_sample() / B::SampleFormat::from_usize(10).unwrap(),
                )
                .into(),
            })
            .build_global(Chip8Audio {
                sound_timer: sound_timer.clone(),
            })
    }
}

pub struct Chip8AudioDataCallback<S: Sample> {
    sound_timer: Arc<RwLock<u8>>,
    square_wave: Mutex<SquareWave<S, 2>>,
}

impl<S: Sample> AudioCallback<S> for Chip8AudioDataCallback<S>
where
    S: FromSample<f32>,
{
    fn generate_audio(&self) -> Box<dyn Iterator<Item = nalgebra::SVector<S, 2>> + '_> {
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
