use crate::scheduler::Task;
use num::rational::Ratio;
use std::{boxed::Box, vec::Vec};

#[derive(Default)]
pub struct TaskMetadata {
    pub global_tasks: Vec<(Ratio<u32>, Box<dyn Task + Send>)>,
    pub tasks: Vec<(Ratio<u32>, Box<dyn Task>)>,
}
