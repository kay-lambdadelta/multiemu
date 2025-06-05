use crate::{rendering_backend::RenderingBackendState, runtime::state::MainRuntime};
use multiemu_runtime::Machine;
use num::rational::Ratio;
use std::{
    fmt::Debug,
    sync::{Arc, RwLock},
};

#[cfg(all(
    any(target_family = "unix", target_os = "windows"),
    not(target_os = "horizon")
))]
pub mod desktop;
#[cfg(all(
    any(target_family = "unix", target_os = "windows"),
    not(target_os = "horizon")
))]
pub use desktop::renderer::software::SoftwareRenderingRuntime;

#[cfg(target_os = "horizon")]
pub mod nintendo_3ds;
#[cfg(target_os = "horizon")]
pub use nintendo_3ds::renderer::software::SoftwareRenderingRuntime;

pub mod state;

pub type MaybeMachine = RwLock<Option<Machine>>;

/// A runtime for a given platform
pub trait Platform<RS: RenderingBackendState, AR: AudioRuntime>: Debug {
    fn run(runtime: MainRuntime<RS, AR>) -> Result<(), Box<dyn std::error::Error>>;
}

pub trait AudioRuntime: Debug {
    // We use a weak pointer so drop doesn't get called here from another thread
    fn new(machine: Arc<MaybeMachine>) -> Self;
    fn sample_rate(&self) -> Ratio<u32>;
    fn pause(&self);
    fn play(&self);
}
