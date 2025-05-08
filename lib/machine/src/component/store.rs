use rustc_hash::FxBuildHasher;

use super::{
    Component, ComponentId, component_ref::ComponentRef, main_thread_queue::MainThreadExecutor,
};
use std::{
    any::Any,
    borrow::Cow,
    cell::{LazyCell, RefCell},
    collections::HashMap,
    fmt::Debug,
    sync::Arc,
};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Could not find component")]
    ComponentNotFound,
    #[error("Component could not be interact with due to its position")]
    ComponentUnreachable,
}

enum ComponentLocation {
    MainThread,
    Global(Arc<dyn Component + Send + Sync>),
}

impl Debug for ComponentLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComponentLocation::MainThread => f.write_str("MainThread"),
            ComponentLocation::Global(_) => f.write_str("Global"),
        }
    }
}

thread_local! {
    pub(super) static IS_MAIN_THREAD: RefCell<bool> = const { RefCell::new(false) };
    static MAIN_THREAD_COMPONENT_STORE: LazyCell<RefCell<HashMap<ComponentId, Arc<dyn Component>>>> = const { LazyCell::new(RefCell::default) };
}

#[derive(Debug)]
pub struct ComponentStore
// This absolutely has to be thread-safe
where
    Self: Send + Sync,
{
    component_ids: scc::HashMap<Cow<'static, str>, ComponentId, FxBuildHasher>,
    component_location: scc::HashMap<ComponentId, ComponentLocation, FxBuildHasher>,
    pub main_thread_queue: Arc<MainThreadExecutor>,
}

impl Default for ComponentStore {
    fn default() -> Self {
        Self::new()
    }
}

impl ComponentStore {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        IS_MAIN_THREAD.with(|is_main_thread| {
            *is_main_thread.borrow_mut() = true;
        });

        MAIN_THREAD_COMPONENT_STORE.with(|task_component_store| {
            let mut task_component_store = task_component_store.borrow_mut();
            task_component_store.clear();
        });

        Self {
            component_ids: scc::HashMap::default(),
            component_location: scc::HashMap::default(),
            main_thread_queue: Arc::new(MainThreadExecutor::default()),
        }
    }

    pub(crate) fn insert_component(
        &self,
        manifest_name: &'static str,
        component_id: ComponentId,
        component: impl Component,
    ) {
        IS_MAIN_THREAD.with(|is_main_thread| {
            assert!(*is_main_thread.borrow());
        });

        MAIN_THREAD_COMPONENT_STORE.with(|task_component_store| {
            let mut task_component_store = task_component_store.borrow_mut();
            task_component_store.insert(component_id, Arc::new(component));
        });

        self.component_ids
            .insert(Cow::Borrowed(manifest_name), component_id)
            .unwrap();

        self.component_location
            .insert(component_id, ComponentLocation::MainThread)
            .unwrap();
    }

    pub(crate) fn insert_component_global(
        &self,
        manifest_name: &'static str,
        component_id: ComponentId,
        component: impl Component + Send + Sync,
    ) {
        IS_MAIN_THREAD.with(|is_main_thread| {
            assert!(*is_main_thread.borrow());
        });

        self.component_ids
            .insert(Cow::Borrowed(manifest_name), component_id)
            .unwrap();

        self.component_location
            .insert(component_id, ComponentLocation::Global(Arc::new(component)))
            .unwrap();
    }

    #[inline]
    // Interacts with a component wherever it may be
    pub(crate) fn interact_dyn(
        &self,
        component_id: ComponentId,
        callback: impl FnOnce(&dyn Component) + Send,
    ) -> Result<(), Error> {
        let is_main_thread = IS_MAIN_THREAD.with(|is_main_thread| *is_main_thread.borrow());

        self.component_location
            .read(&component_id, |_, location| {
                match location {
                    ComponentLocation::MainThread => {
                        if is_main_thread {
                            MAIN_THREAD_COMPONENT_STORE.with(|thread_component_store| {
                                let thread_component_store = thread_component_store.borrow();
                                let component = thread_component_store.get(&component_id).unwrap();
                                callback(component.as_ref());
                            });
                        } else {
                            self.main_thread_queue.wait_on_main(|| {
                                MAIN_THREAD_COMPONENT_STORE.with(|thread_component_store| {
                                    let thread_component_store = thread_component_store.borrow();
                                    let component =
                                        thread_component_store.get(&component_id).unwrap();
                                    callback(component.as_ref());
                                });
                            });
                        }
                    }
                    ComponentLocation::Global(component) => {
                        callback(component.as_ref());
                    }
                }

                Ok(())
            })
            .ok_or(Error::ComponentNotFound)?
    }

    #[inline]
    // Interacts with a component but restricted to the local instance, mostly for graphics initialization
    pub(crate) fn interact_dyn_local(
        &self,
        component_id: ComponentId,
        callback: impl FnOnce(&dyn Component),
    ) -> Result<(), Error> {
        let is_main_thread = IS_MAIN_THREAD.with(|is_main_thread| *is_main_thread.borrow());

        self.component_location
            .read(&component_id, |_, location| {
                match location {
                    ComponentLocation::MainThread => {
                        if is_main_thread {
                            MAIN_THREAD_COMPONENT_STORE.with(|thread_component_store| {
                                let thread_component_store = thread_component_store.borrow();
                                let component = thread_component_store.get(&component_id).unwrap();
                                callback(component.as_ref());
                            });
                        } else {
                            return Err(Error::ComponentUnreachable);
                        }
                    }
                    ComponentLocation::Global(component) => {
                        callback(component.as_ref());
                    }
                }

                Ok(())
            })
            .ok_or(Error::ComponentNotFound)?
    }

    #[inline]
    pub(crate) fn interact<C: Component>(
        &self,
        component_id: ComponentId,
        callback: impl FnOnce(&C) + Send,
    ) -> Result<(), Error> {
        self.interact_dyn(component_id, |component| {
            let component = (component as &dyn Any).downcast_ref::<C>().unwrap();
            callback(component);
        })
    }

    #[inline]
    pub(crate) fn interact_local<C: Component>(
        &self,
        component_id: ComponentId,
        callback: impl FnOnce(&C),
    ) -> Result<(), Error> {
        self.interact_dyn_local(component_id, |component| {
            let component = (component as &dyn Any).downcast_ref::<C>().unwrap();
            callback(component);
        })
    }

    #[inline]
    pub fn interact_by_name<C: Component>(
        &self,
        manifest_name: &str,
        callback: impl FnOnce(&C) + Send,
    ) -> Result<(), Error> {
        let component_id = *self.component_ids.get(manifest_name).unwrap().get();

        self.interact(component_id, callback)
    }

    #[inline]
    pub fn interact_by_name_local<C: Component>(
        &self,
        manifest_name: &str,
        callback: impl FnOnce(&C),
    ) -> Result<(), Error> {
        let component_id = *self.component_ids.get(manifest_name).unwrap().get();

        self.interact_local(component_id, callback)
    }

    pub fn get<C: Component>(self: &Arc<Self>, manifest_name: &str) -> Option<ComponentRef<C>> {
        let component_id = *self.component_ids.get(manifest_name).unwrap().get();

        let component = if let ComponentLocation::Global(component) =
            self.component_location.get(&component_id)?.get()
        {
            Some(component.clone())
        } else {
            None
        };

        Some(ComponentRef::new(component_id, component, self.clone()))
    }
}
