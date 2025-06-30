use super::{Component, ComponentId};
use crate::{
    component::ComponentRef,
    utils::{Fragile, MainThreadQueue},
};
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
pub enum ComponentStoreError {
    #[error("Could not find component")]
    ComponentNotFound,
    #[error("Component could not be interact with due to its position")]
    ComponentUnreachable,
}

#[derive(Debug)]
enum ComponentLocation {
    Global(Box<dyn Component + Send + Sync>),
    // Use fragile to guard thread safety
    Local(Fragile<Box<dyn Component>>),
}

#[derive(Debug)]
/// The store for components
pub struct ComponentStore
// This absolutely has to be thread-safe
where
    Self: Send + Sync,
{
    component_names: RwLock<HashMap<ComponentId, ComponentName, FxBuildHasher>>,
    component_location: RwLock<HashMap<ComponentId, ComponentLocation, FxBuildHasher>>,
    main_thread_queue: Arc<MainThreadQueue>,
    was_started: AtomicBool,
}

impl ComponentStore {
    pub fn new(main_thread_queue: Arc<MainThreadQueue>) -> Arc<Self> {
        Self {
            component_names: RwLock::default(),
            component_location: RwLock::default(),
            main_thread_queue,
            was_started: AtomicBool::new(false),
        }
        .into()
    }

    pub fn get_name(&self, component_id: ComponentId) -> Option<ComponentName> {
        self.component_names
            .read()
            .unwrap()
            .get(&component_id)
            .cloned()
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

    pub fn insert_component(
        &self,
        name: ComponentName,
        component_id: ComponentId,
        component: impl Component,
    ) {
        let _ = self
            .component_names
            .write()
            .unwrap()
            .insert(component_id, name);

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
        let _ = self
            .component_names
            .write()
            .unwrap()
            .insert(component_id, name);

        if self
            .component_location
            .write()
            .unwrap()
            .insert(component_id, ComponentLocation::Global(Box::new(component)))
            .is_some()
        {
            panic!("Component already exists");
        }
    }

    // Interacts with a component wherever it may be
    #[inline]
    pub fn interact_dyn<T: Send + 'static>(
        &self,
        component_id: ComponentId,
        callback: impl FnOnce(&dyn Component) -> T + Send,
    ) -> Result<T, ComponentStoreError> {
        let component_location_guard = self.component_location.read().unwrap();

        match component_location_guard
            .get(&component_id)
            .ok_or(ComponentStoreError::ComponentNotFound)?
        {
            ComponentLocation::Local(component) => Ok(self
                .main_thread_queue
                .maybe_wait_on_main(|| callback(component.get().unwrap().as_ref()))),
            ComponentLocation::Global(component) => Ok(callback(component.as_ref())),
        }
    }

    // Interacts with a component but restricted to the local instance, mostly for graphics initialization
    #[inline]
    pub fn interact_dyn_local<T: 'static>(
        &self,
        component_id: ComponentId,
        callback: impl FnOnce(&dyn Component) -> T,
    ) -> Result<T, ComponentStoreError> {
        let component_location_guard = self.component_location.read().unwrap();

        match component_location_guard
            .get(&component_id)
            .ok_or(ComponentStoreError::ComponentNotFound)?
        {
            ComponentLocation::Local(component) => Ok(callback(component.get().unwrap().as_ref())),
            ComponentLocation::Global(component) => Ok(callback(component.as_ref())),
        }
    }

    #[inline]
    pub fn interact<C: Component, T: Send + 'static>(
        &self,
        component_id: ComponentId,
        callback: impl FnOnce(&C) -> T + Send,
    ) -> Result<T, ComponentStoreError> {
        self.interact_dyn(component_id, |component| {
            let component = (component as &dyn Any).downcast_ref::<C>().unwrap();
            callback(component)
        })
    }

    #[inline]
    pub fn interact_local<C: Component, T: 'static>(
        &self,
        component_id: ComponentId,
        callback: impl FnOnce(&C) -> T,
    ) -> Result<T, ComponentStoreError> {
        self.interact_dyn_local(component_id, |component| {
            let component = (component as &dyn Any).downcast_ref::<C>().unwrap();
            callback(component)
        })
    }

    pub fn get<C: Component>(self: &Arc<Self>, id: ComponentId) -> Option<ComponentRef<C>> {
        if self.component_location.read().unwrap().contains_key(&id) {
            Some(ComponentRef::new(self.clone(), id))
        } else {
            None
        }
    }
}
