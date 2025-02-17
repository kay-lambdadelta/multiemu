use super::ComponentBuilder;
use crate::{
    component::Component,
    scheduler::{task::Task, StoredTask},
};
use num::rational::Ratio;

pub struct TaskMetadata {
    pub frequency: Ratio<u64>,
    pub task: StoredTask,
}

impl<C: Component> ComponentBuilder<'_, C> {
    pub fn insert_task(mut self, frequency: Ratio<u64>, mut callback: impl Task<C>) -> Self {
        self.component_metadata.task = Some(TaskMetadata {
            frequency,
            task: Box::new(move |component, period| {
                let component = component.as_any().downcast_ref::<C>().unwrap();
                callback.run(component, period);
            }),
        });

        self
    }
}
