use crate::component::Component;
use num::rational::Ratio;
use std::{boxed::Box, num::NonZero, vec::Vec};

#[derive(Default)]
pub struct TaskMetadata {
    #[allow(clippy::type_complexity)]
    pub tasks: Vec<(
        Ratio<u32>,
        Box<dyn FnMut(&dyn Component, NonZero<u32>) + Send + 'static>,
    )>,
}
