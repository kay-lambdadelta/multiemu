use super::ComponentBuilder;
use crate::{component::Component, scheduler::task::Task};
use num::rational::Ratio;
use std::{any::Any, num::NonZero};

#[derive(Default)]
pub struct TaskMetadata {
    #[allow(clippy::type_complexity)]
    pub tasks: Vec<(
        Ratio<u32>,
        Box<dyn FnMut(&dyn Component, NonZero<u32>) + Send + 'static>,
    )>,
}

impl<C: Component> ComponentBuilder<'_, C> {
    pub fn insert_task(mut self, frequency: Ratio<u32>, mut callback: impl Task<C>) -> Self {
        let task_metatada = self.component_metadata.task.get_or_insert_default();

        task_metatada.tasks.push((
            frequency,
            Box::new(move |component, period| {
                let component = (component as &dyn Any).downcast_ref::<C>().unwrap();
                callback.run(component, period);
            }),
        ));

        self
    }
}
