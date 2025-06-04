use crate::{rendering_backend::RenderingBackendState, runtime::state::WindowingRuntime};
use multiemu_runtime::Machine;
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

pub type MaybeMachine = Arc<RwLock<Option<Machine>>>;

/// A runtime for a given platform
pub trait Platform<RS: RenderingBackendState>: Debug {
    fn run(runtime: WindowingRuntime<RS>) -> Result<(), Box<dyn std::error::Error>>;
}

pub trait AudioRuntime: Debug {
    fn new(machine: MaybeMachine) -> Self;
    fn pause(&self);
    fn play(&self);
}
