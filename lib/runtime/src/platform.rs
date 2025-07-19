use crate::utils::{DirectMainThreadExecutor, MainThreadExecutor};
use multiemu_audio::SampleFormat;
use multiemu_graphics::{GraphicsApi, software::Software};
use std::fmt::Debug;

/// A trait abstracting over the various things the platform requires
pub trait Platform: Debug + 'static {
    /// Sample format this platform uses
    type SampleFormat: SampleFormat;
    /// Main thread executor this platform uses, for in runtime use
    type MainThreadExecutor: MainThreadExecutor;
    /// Graphics api in use
    type GraphicsApi: GraphicsApi;
}

#[derive(Debug)]
/// Test platform
pub struct TestPlatform;

impl Platform for TestPlatform {
    type SampleFormat = f32;
    type MainThreadExecutor = DirectMainThreadExecutor;
    type GraphicsApi = Software;
}
