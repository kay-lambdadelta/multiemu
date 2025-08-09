use crate::scheduler::{Task, TaskName};
use num::rational::Ratio;
use std::{boxed::Box, collections::HashMap, fmt::Debug};

pub struct StoredTask {
    pub period: Ratio<u32>,
    pub task: Box<dyn Task>,
}

impl Debug for StoredTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoredTask")
            .field("frequency", &self.period)
            .finish()
    }
}

#[derive(Default, Debug)]
pub struct TaskMetadata {
    pub tasks: HashMap<TaskName, StoredTask>,
}
