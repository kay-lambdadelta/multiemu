use crate::scheduler::Task;
use num::rational::Ratio;
use std::{boxed::Box, vec::Vec};

pub struct StoredTask {
    pub frequency: Ratio<u32>,
    pub lazy: bool,
    pub task: Box<dyn Task>,
}

#[derive(Default)]
pub struct TaskMetadata {
    pub tasks: Vec<StoredTask>,
}
