use super::{Component, ComponentId};
use crate::utils::{Fragile, MainThreadQueue, is_main_thread};
use multiemu_save::ComponentName;
use rustc_hash::FxBuildHasher;
use std::{
    any::Any,
    boxed::Box,
    collections::HashMap,
    fmt::Debug,
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Could not find component")]
    ComponentNotFound,
    #[error("Component could not be interact with due to its position")]
    ComponentUnreachable,
}

enum ComponentLocation {
    Global(Arc<dyn Component + Send + Sync>),
    // Use fragile to guard thread safety
    Local(Fragile<Box<dyn Component>>),
}

impl Debug for ComponentLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComponentLocation::Global(_) => f.write_str("Global"),
            ComponentLocation::Local(_) => f.write_str("Local"),
        }
    }
}

#[derive(Debug)]
pub struct ComponentStore
// This absolutely has to be thread-safe
where
    Self: Send + Sync,
{
    component_ids: RwLock<HashMap<ComponentName, ComponentId, FxBuildHasher>>,
    component_location: RwLock<HashMap<ComponentId, ComponentLocation, FxBuildHasher>>,
    pub(crate) main_thread_queue: Arc<MainThreadQueue>,
    was_started: AtomicBool,
}

impl Default for ComponentStore {
    fn default() -> Self {
        Self::new()
    }
}

impl ComponentStore {
    // for our purposes the calling thread is the main thread
    pub fn new() -> Self {
        assert!(is_main_thread());

        Self {
            component_ids: RwLock::default(),
            component_location: RwLock::default(),
            main_thread_queue: Arc::new(MainThreadQueue::default()),
            was_started: AtomicBool::new(false),
        }
    }

    pub fn interact_all(&self, mut callback: impl FnMut(&dyn Component) + Send) {
        if self.was_started.swap(true, Ordering::SeqCst) {
            panic!("Machine already started");
        }

        let component_location_guard = self.component_location.read().unwrap();

        for component in component_location_guard.values() {
            match component {
                ComponentLocation::Global(component) => callback(component.as_ref()),
                ComponentLocation::Local(component) => self
                    .main_thread_queue
                    .maybe_wait_on_main(|| callback(component.get().unwrap().as_ref())),
            }
        }
    }

    pub(crate) fn insert_component(
        &self,
        name: ComponentName,
        component_id: ComponentId,
        component: impl Component,
    ) {
        assert!(is_main_thread());

        let _ = self
            .component_ids
            .write()
            .unwrap()
            .insert(name, component_id);

        if self
            .component_location
            .write()
            .unwrap()
            .insert(
                component_id,
                ComponentLocation::Local(Fragile::new(Box::new(component))),
            )
            .is_some()
        {
            panic!("Component already exists");
        }
    }

    pub(crate) fn insert_component_global(
        &self,
        name: ComponentName,
        component_id: ComponentId,
        component: impl Component + Send + Sync,
    ) {
        assert!(is_main_thread());

        let _ = self
            .component_ids
            .write()
            .unwrap()
            .insert(name, component_id);

        if self
            .component_location
            .write()
            .unwrap()
            .insert(component_id, ComponentLocation::Global(Arc::new(component)))
            .is_some()
        {
            panic!("Component already exists");
        }
    }

    #[inline]
    // Interacts with a component wherever it may be
    pub(crate) fn interact_dyn<T: Send + 'static>(
        &self,
        component_id: ComponentId,
        callback: impl FnOnce(&dyn Component) -> T + Send,
    ) -> Result<T, Error> {
        let component_location_guard = self.component_location.read().unwrap();

        match component_location_guard
            .get(&component_id)
            .ok_or(Error::ComponentNotFound)?
        {
            ComponentLocation::Local(component) => Ok(self
                .main_thread_queue
                .maybe_wait_on_main(|| callback(component.get().unwrap().as_ref()))),
            ComponentLocation::Global(component) => Ok(callback(component.as_ref())),
        }
    }

    #[inline]
    // Interacts with a component but restricted to the local instance, mostly for graphics initialization
    pub(crate) fn interact_dyn_local<T: 'static>(
        &self,
        component_id: ComponentId,
        callback: impl FnOnce(&dyn Component) -> T,
    ) -> Result<T, Error> {
        assert!(is_main_thread());

        let component_location_guard = self.component_location.read().unwrap();

        match component_location_guard
            .get(&component_id)
            .ok_or(Error::ComponentNotFound)?
        {
            ComponentLocation::Local(component) => Ok(callback(component.get().unwrap().as_ref())),
            ComponentLocation::Global(component) => Ok(callback(component.as_ref())),
        }
    }

    #[inline]
    pub(crate) fn interact<C: Component, T: Send + 'static>(
        &self,
        component_id: ComponentId,
        callback: impl FnOnce(&C) -> T + Send,
    ) -> Result<T, Error> {
        self.interact_dyn(component_id, |component| {
            let component = (component as &dyn Any).downcast_ref::<C>().unwrap();
            callback(component)
        })
    }

    #[inline]
    pub(crate) fn interact_local<C: Component, T: 'static>(
        &self,
        component_id: ComponentId,
        callback: impl FnOnce(&C) -> T,
    ) -> Result<T, Error> {
        self.interact_dyn_local(component_id, |component| {
            let component = (component as &dyn Any).downcast_ref::<C>().unwrap();
            callback(component)
        })
    }
}
