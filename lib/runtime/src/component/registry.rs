use super::{Component, ComponentId};
use crate::{
    component::ComponentPath,
    utils::{Fragile, MainThreadQueue},
};
use rustc_hash::FxBuildHasher;
use std::{
    any::Any,
    boxed::Box,
    cell::RefCell,
    fmt::Debug,
    ops::{Deref, DerefMut},
    sync::{Arc, RwLock},
};

#[derive(thiserror::Error, Debug)]
pub enum ComponentStoreError {
    #[error("Could not find component")]
    ComponentNotFound,
    #[error("Component could not be interact with due to its position")]
    ComponentUnreachable,
}

#[derive(Debug)]
enum ComponentStorage {
    Global(Arc<RwLock<dyn Component + Send + Sync>>),
    // Use fragile to guard thread safety
    Local(Fragile<RefCell<Box<dyn Component>>>),
}

#[derive(Debug)]
struct ComponentInfo {
    storage: ComponentStorage,
    path: ComponentPath,
}

#[derive(Debug)]
/// The store for components
pub struct ComponentRegistry
// This absolutely has to be thread-safe
where
    Self: Send + Sync,
{
    components: boxcar::Vec<ComponentInfo>,
    component_ids: scc::HashMap<ComponentPath, ComponentId, FxBuildHasher>,
    main_thread_queue: Arc<MainThreadQueue>,
}

impl ComponentRegistry {
    pub fn new(main_thread_queue: Arc<MainThreadQueue>) -> Arc<Self> {
        Self {
            components: boxcar::Vec::new(),
            component_ids: scc::HashMap::default(),
            main_thread_queue,
        }
        .into()
    }

    pub fn get_path(&self, component_id: ComponentId) -> ComponentPath {
        self.components[component_id.get() as usize].path.clone()
    }

    pub fn get_id(&self, path: &ComponentPath) -> Option<ComponentId> {
        self.component_ids.get_sync(path).as_deref().copied()
    }

    pub(crate) fn interact_all(
        &self,
        mut callback: impl FnMut(&ComponentPath, &dyn Component) + Send,
    ) {
        for component_info in self.components.iter().map(|(_, component)| component) {
            match component_info.storage.read().unwrap().deref() {
                ComponentStorage::Global(component) => {
                    callback(&component_info.path, component.as_ref())
                }
                ComponentStorage::Local(component) => {
                    self.main_thread_queue.maybe_wait_on_main(|| {
                        callback(&component_info.path, component.get().unwrap().as_ref())
                    })
                }
            }
        }
    }

    pub(crate) fn interact_all_mut(
        &self,
        mut callback: impl FnMut(&ComponentPath, &mut dyn Component) + Send,
    ) {
        for component_info in self.components.iter().map(|(_, component)| component) {
            match component_info.storage.write().unwrap().deref_mut() {
                ComponentStorage::Global(component) => {
                    callback(&component_info.path, component.as_mut())
                }
                ComponentStorage::Local(component) => {
                    self.main_thread_queue.maybe_wait_on_main(|| {
                        callback(&component_info.path, component.get_mut().unwrap().as_mut())
                    })
                }
            }
        }
    }

    pub(crate) fn insert_component_local(
        &self,
        path: ComponentPath,
        component: impl Component,
    ) -> ComponentId {
        let storage = RwLock::new(ComponentStorage::Local(Fragile::new(Box::new(component))));

        let index = self.components.push(ComponentInfo {
            storage,
            path: path.clone(),
        });

        let component_id = ComponentId::new(index.try_into().unwrap());
        let _ = self.component_ids.insert_sync(path, component_id);

        component_id
    }

    pub(crate) fn insert_component(
        &self,
        path: ComponentPath,
        component: impl Component + Send + Sync,
    ) -> ComponentId {
        let storage = RwLock::new(ComponentStorage::Global(Box::new(component)));

        let index = self.components.push(ComponentInfo {
            storage,
            path: path.clone(),
        });

        let component_id = ComponentId::new(index.try_into().unwrap());
        let _ = self.component_ids.insert_sync(path, component_id);

        component_id
    }

    // Interacts with a component wherever it may be
    #[inline]
    pub fn interact_dyn<T: Send + 'static>(
        &self,
        component_id: ComponentId,
        callback: impl FnOnce(&dyn Component) -> T + Send,
    ) -> Result<T, ComponentStoreError> {
        match self.components[component_id.get() as usize]
            .storage
            .read()
            .unwrap()
            .deref()
        {
            ComponentStorage::Local(component) => self
                .main_thread_queue
                .maybe_wait_on_main(|| Ok(callback(component.get().unwrap().as_ref()))),
            ComponentStorage::Global(component) => Ok(callback(component.as_ref())),
        }
    }

    #[inline]
    pub fn interact_dyn_mut<T: Send + 'static>(
        &self,
        component_id: ComponentId,
        callback: impl FnOnce(&mut dyn Component) -> T + Send,
    ) -> Result<T, ComponentStoreError> {
        match self.components[component_id.get() as usize]
            .storage
            .write()
            .unwrap()
            .deref_mut()
        {
            ComponentStorage::Local(component) => self
                .main_thread_queue
                .maybe_wait_on_main(|| Ok(callback(component.get_mut().unwrap().as_mut()))),
            ComponentStorage::Global(component) => Ok(callback(component.as_mut())),
        }
    }

    // Interacts with a component but restricted to the local instance, mostly for graphics initialization
    #[inline]
    pub fn interact_dyn_local<T: 'static>(
        &self,
        component_id: ComponentId,
        callback: impl FnOnce(&dyn Component) -> T,
    ) -> Result<T, ComponentStoreError> {
        match self.components[component_id.get() as usize]
            .storage
            .read()
            .unwrap()
            .deref()
        {
            ComponentStorage::Local(component) => Ok(callback(component.get().unwrap().as_ref())),
            ComponentStorage::Global(component) => Ok(callback(component.as_ref())),
        }
    }

    #[inline]
    pub fn interact_dyn_local_mut<T: 'static>(
        &self,
        component_id: ComponentId,
        callback: impl FnOnce(&mut dyn Component) -> T,
    ) -> Result<T, ComponentStoreError> {
        match self.components[component_id.get() as usize]
            .storage
            .write()
            .unwrap()
            .deref_mut()
        {
            ComponentStorage::Local(component) => {
                Ok(callback(component.get_mut().unwrap().as_mut()))
            }
            ComponentStorage::Global(component) => Ok(callback(component.as_mut())),
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
    pub fn interact_mut<C: Component, T: Send + 'static>(
        &self,
        component_id: ComponentId,
        callback: impl FnOnce(&mut C) -> T + Send,
    ) -> Result<T, ComponentStoreError> {
        self.interact_dyn_mut(component_id, |component| {
            let component = (component as &mut dyn Any).downcast_mut::<C>().unwrap();
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

    #[inline]
    pub fn interact_local_mut<C: Component, T: 'static>(
        &self,
        component_id: ComponentId,
        callback: impl FnOnce(&mut C) -> T,
    ) -> Result<T, ComponentStoreError> {
        self.interact_dyn_local_mut(component_id, |component| {
            let component = (component as &mut dyn Any).downcast_mut::<C>().unwrap();
            callback(component)
        })
    }
}
