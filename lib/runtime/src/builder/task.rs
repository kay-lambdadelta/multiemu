use crate::scheduler::Task;
use num::rational::Ratio;
use std::{boxed::Box, fmt::Debug, vec::Vec};

pub struct StoredTask {
    pub period: Ratio<u32>,
    pub lazy: bool,
    pub task: Box<dyn Task>,
}

impl Debug for StoredTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoredTask")
            .field("frequency", &self.period)
            .field("lazy", &self.lazy)
            .finish()
    }
}

#[derive(Default)]
pub struct TaskMetadata {
    pub tasks: Vec<StoredTask>,
}
