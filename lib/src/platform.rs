use crate::{
    graphics::{GraphicsApi, software::Software},
    utils::{DirectMainThreadExecutor, MainThreadExecutor},
};
use std::fmt::Debug;

/// A trait abstracting over the various things the platform requires
pub trait Platform: Debug + 'static {
    /// Main thread executor this platform uses, for in runtime use
    type MainThreadExecutor: MainThreadExecutor;
    /// Graphics api in use
    type GraphicsApi: GraphicsApi;
}

#[derive(Debug)]
/// Test platform
pub struct TestPlatform;

impl Platform for TestPlatform {
    type MainThreadExecutor = DirectMainThreadExecutor;
    type GraphicsApi = Software;
}
