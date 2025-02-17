use super::{component_ref::ComponentRef, Component, ComponentId};
use crossbeam::channel::Sender;
use std::{
    cell::{OnceCell, RefCell},
    collections::HashMap,
    fmt::Debug,
    sync::Arc,
};
use strum::Display;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Display)]
pub enum TaskId {
    Main,
    Worker(u8),
}

enum ComponentLocation {
    Global(Arc<dyn Component + Send + Sync>),
    Task(TaskId),
}

impl Debug for ComponentLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComponentLocation::Global { .. } => write!(f, "Global"),
            ComponentLocation::Task(task_id) => write!(f, "Task({task_id})"),
        }
    }
}

thread_local! {
    static WORKER_TASK_ID: OnceCell<TaskId> = const { OnceCell::new() };
    static MAIN_THREAD_COMPONENT_STORE: OnceCell<RefCell<HashMap<ComponentId, Arc<dyn Component>>>> =
        OnceCell::new();
}

#[derive(Default, Debug)]
pub struct ComponentStore
// This absolutely has to be thread-safe
where
    Self: Send + Sync,
{
    component_location: scc::HashMap<ComponentId, ComponentLocation>,
    external_executors: scc::HashMap<u8, Sender<Box<dyn FnOnce(&dyn Component) + Send>>>,
}

impl ComponentStore {
    pub(crate) fn insert_component(&self, component_id: ComponentId, component: impl Component) {
        MAIN_THREAD_COMPONENT_STORE.with(|task_component_store| {
            let mut task_component_store = task_component_store
                .get_or_init(RefCell::default)
                .borrow_mut();
            task_component_store.insert(component_id, Arc::new(component));
        });

        self.component_location
            .insert(component_id, ComponentLocation::Task(TaskId::Main));
    }

    pub(crate) fn insert_component_global(
        &mut self,
        component_id: ComponentId,
        component: impl Component + Send + Sync,
    ) {
        self.component_location
            .insert(component_id, ComponentLocation::Global(Arc::new(component)));
    }

    #[inline]
    pub(crate) fn interact_dyn(
        &self,
        component_id: ComponentId,
        callback: impl FnOnce(&dyn Component) + Send,
    ) {
        if let Some(location) = self.component_location.get(&component_id) {
            match location.get() {
                ComponentLocation::Task(TaskId::Main) => {
                    MAIN_THREAD_COMPONENT_STORE.with(|thread_component_store| {
                        let thread_component_store = thread_component_store.get().unwrap().borrow();
                        let component = thread_component_store.get(&component_id).unwrap();
                        callback(component.as_ref());
                    });
                }
                ComponentLocation::Global(component) => {
                    callback(component.as_ref());
                }
                ComponentLocation::Task(TaskId::Worker(worker_id)) => {
                    todo!()
                }
            }
        }
    }

    #[inline]
    pub(super) fn interact<C: Component>(
        &self,
        component_id: ComponentId,
        callback: impl FnOnce(&C) + Send,
    ) {
        self.interact_dyn(component_id, |component| {
            let component = component.as_any().downcast_ref::<C>().unwrap();
            callback(component);
        });
    }

    pub fn get_compoonent<C: Component>(
        self: &Arc<Self>,
        component_id: ComponentId,
    ) -> Option<ComponentRef<C>> {
        let component = if let ComponentLocation::Global(component) =
            self.component_location.get(&component_id)?.get()
        {
            Some(component.clone())
        } else {
            None
        };

        Some(ComponentRef::new(component_id, component, self.clone()))
    }

    pub fn execute_task(&self, callback: impl FnOnce(Box<dyn FnOnce() + Send>)) {
        let local_task_id = WORKER_TASK_ID
            .with(|local_task_id| *local_task_id.get().expect("Task was not assigned an id"));
    }
}
