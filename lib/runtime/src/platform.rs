use std::fmt::Debug;

use crate::graphics::{GraphicsApi, software::Software};

/// A trait abstracting over the various things the platform requires
pub trait Platform: Debug + 'static {
    /// Graphics api in use
    type GraphicsApi: GraphicsApi;
}

#[derive(Debug)]
/// Test platform
pub struct TestPlatform;

impl Platform for TestPlatform {
    type GraphicsApi = Software;
}
