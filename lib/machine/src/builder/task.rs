use super::ComponentBuilder;
use crate::{component::Component, scheduler::task::Task};
use num::rational::Ratio;

#[derive(Default)]
pub struct TaskMetadata {
    pub tasks: Vec<(
        Ratio<u64>,
        Box<dyn FnMut(&dyn Component, u64) + Send + 'static>,
    )>,
}

impl<C: Component> ComponentBuilder<'_, C> {
    pub fn insert_task(mut self, frequency: Ratio<u64>, mut callback: impl Task<C> + Send) -> Self {
        let task_metatada = self.component_metadata.task.get_or_insert_default();

        task_metatada.tasks.push((
            frequency,
            Box::new(move |component, period| {
                let component = component.as_any().downcast_ref::<C>().unwrap();
                callback.run(component, period);
            }),
        ));

        self
    }
}
