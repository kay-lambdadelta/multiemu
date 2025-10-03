use crate::{
    component::{Component, ComponentId, ComponentPath},
    utils::{Fragile, MainThreadQueue},
};
use rustc_hash::FxBuildHasher;
use std::{
    any::Any,
    boxed::Box,
    cell::RefCell,
    collections::HashMap,
    fmt::Debug,
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
    components: Vec<Option<ComponentInfo>>,
    component_ids: HashMap<ComponentPath, ComponentId, FxBuildHasher>,
    main_thread_queue: Arc<MainThreadQueue>,
}

impl ComponentRegistry {
    pub fn new(main_thread_queue: Arc<MainThreadQueue>) -> Self {
        Self {
            components: Vec::new(),
            component_ids: HashMap::default(),
            main_thread_queue,
        }
    }

    pub fn get_path(&self, component_id: ComponentId) -> ComponentPath {
        self.components[component_id.get() as usize]
            .as_ref()
            .unwrap()
            .path
            .clone()
    }

    pub fn get_id(&self, path: &ComponentPath) -> Option<ComponentId> {
        self.component_ids.get(path).as_deref().copied()
    }

    pub(crate) fn interact_all(
        &self,
        mut callback: impl FnMut(&ComponentPath, &dyn Component) + Send,
    ) {
        for component_info in self.components.iter() {
            let component_info = component_info.as_ref().unwrap();

            match &component_info.storage {
                ComponentStorage::Global(component) => {
                    callback(&component_info.path, &*component.read().unwrap())
                }
                ComponentStorage::Local(component) => {
                    self.main_thread_queue.maybe_wait_on_main(|| {
                        callback(&component_info.path, &**component.get().unwrap().borrow())
                    })
                }
            }
        }
    }

    pub(crate) fn interact_all_mut(
        &self,
        mut callback: impl FnMut(&ComponentPath, &mut dyn Component) + Send,
    ) {
        for component_info in self.components.iter() {
            let component_info = component_info.as_ref().unwrap();

            match &component_info.storage {
                ComponentStorage::Global(component) => {
                    callback(&component_info.path, &mut *component.write().unwrap())
                }
                ComponentStorage::Local(component) => {
                    self.main_thread_queue.maybe_wait_on_main(|| {
                        callback(
                            &component_info.path,
                            &mut **component.get().unwrap().borrow_mut(),
                        )
                    })
                }
            }
        }
    }

    pub(crate) fn reserve_component(&mut self, path: ComponentPath) {
        let index = self.components.len();
        self.components.push(None);

        let component_id = ComponentId::new(index.try_into().unwrap());
        self.component_ids.insert(path, component_id);
    }

    pub(crate) fn insert_component_local(
        &mut self,
        path: ComponentPath,
        component: impl Component,
    ) {
        let storage = ComponentStorage::Local(Fragile::new(RefCell::new(Box::new(component))));
        let id = self.component_ids.get(&path).unwrap();

        self.components[id.get() as usize] = Some(ComponentInfo { storage, path });
    }

    pub(crate) fn insert_component(
        &mut self,
        path: ComponentPath,
        component: impl Component + Send + Sync,
    ) {
        let storage = ComponentStorage::Global(Arc::new(RwLock::new(component)));

        let id = self.component_ids.get(&path).unwrap();
        self.components[id.get() as usize] = Some(ComponentInfo { storage, path });
    }

    // Interacts with a component wherever it may be
    #[inline]
    pub fn interact_dyn<T: Send + 'static>(
        &self,
        component_id: ComponentId,
        callback: impl FnOnce(&dyn Component) -> T + Send,
    ) -> Result<T, ComponentStoreError> {
        let component_info = self.components[component_id.get() as usize]
            .as_ref()
            .unwrap();

        match &component_info.storage {
            ComponentStorage::Local(component) => self
                .main_thread_queue
                .maybe_wait_on_main(|| Ok(callback(&**component.get().unwrap().borrow()))),
            ComponentStorage::Global(component) => Ok(callback(&*component.read().unwrap())),
        }
    }

    #[inline]
    pub fn interact_dyn_mut<T: Send + 'static>(
        &self,
        component_id: ComponentId,
        callback: impl FnOnce(&mut dyn Component) -> T + Send,
    ) -> Result<T, ComponentStoreError> {
        let component_info = self.components[component_id.get() as usize]
            .as_ref()
            .unwrap();

        match &component_info.storage {
            ComponentStorage::Local(component) => self
                .main_thread_queue
                .maybe_wait_on_main(|| Ok(callback(&mut **component.get().unwrap().borrow_mut()))),
            ComponentStorage::Global(component) => Ok(callback(&mut *component.write().unwrap())),
        }
    }

    // Interacts with a component but restricted to the local instance, mostly for graphics initialization
    #[inline]
    pub fn interact_dyn_local<T: 'static>(
        &self,
        component_id: ComponentId,
        callback: impl FnOnce(&dyn Component) -> T,
    ) -> Result<T, ComponentStoreError> {
        let component_info = self.components[component_id.get() as usize]
            .as_ref()
            .unwrap();

        match &component_info.storage {
            ComponentStorage::Local(component) => {
                Ok(callback(&**component.get().unwrap().borrow()))
            }
            ComponentStorage::Global(component) => Ok(callback(&*component.read().unwrap())),
        }
    }

    #[inline]
    pub fn interact_dyn_local_mut<T: 'static>(
        &self,
        component_id: ComponentId,
        callback: impl FnOnce(&mut dyn Component) -> T,
    ) -> Result<T, ComponentStoreError> {
        let component_info = self.components[component_id.get() as usize]
            .as_ref()
            .unwrap();

        match &component_info.storage {
            ComponentStorage::Local(component) => {
                Ok(callback(&mut **component.get().unwrap().borrow_mut()))
            }
            ComponentStorage::Global(component) => Ok(callback(&mut *component.write().unwrap())),
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

    #[inline]
    pub fn interact_by_path<C: Component, T: Send + 'static>(
        &self,
        path: &ComponentPath,
        callback: impl FnOnce(&C) -> T + Send,
    ) -> Result<T, ComponentStoreError> {
        let id = self
            .get_id(path)
            .ok_or(ComponentStoreError::ComponentNotFound)?;
        self.interact(id, callback)
    }

    #[inline]
    pub fn interact_mut_by_path<C: Component, T: Send + 'static>(
        &self,
        path: &ComponentPath,
        callback: impl FnOnce(&mut C) -> T + Send,
    ) -> Result<T, ComponentStoreError> {
        let id = self
            .get_id(path)
            .ok_or(ComponentStoreError::ComponentNotFound)?;
        self.interact_mut(id, callback)
    }

    #[inline]
    pub fn interact_local_by_path<C: Component, T: 'static>(
        &self,
        path: &ComponentPath,
        callback: impl FnOnce(&C) -> T,
    ) -> Result<T, ComponentStoreError> {
        let id = self
            .get_id(path)
            .ok_or(ComponentStoreError::ComponentNotFound)?;
        self.interact_local(id, callback)
    }

    #[inline]
    pub fn interact_local_mut_by_path<C: Component, T: 'static>(
        &self,
        path: &ComponentPath,
        callback: impl FnOnce(&mut C) -> T,
    ) -> Result<T, ComponentStoreError> {
        let id = self
            .get_id(path)
            .ok_or(ComponentStoreError::ComponentNotFound)?;
        self.interact_local_mut(id, callback)
    }

    #[inline]
    pub fn interact_dyn_by_path<T: Send + 'static>(
        &self,
        path: &ComponentPath,
        callback: impl FnOnce(&dyn Component) -> T + Send,
    ) -> Result<T, ComponentStoreError> {
        let id = self
            .get_id(path)
            .ok_or(ComponentStoreError::ComponentNotFound)?;
        self.interact_dyn(id, callback)
    }

    #[inline]
    pub fn interact_dyn_mut_by_path<T: Send + 'static>(
        &self,
        path: &ComponentPath,
        callback: impl FnOnce(&mut dyn Component) -> T + Send,
    ) -> Result<T, ComponentStoreError> {
        let id = self
            .get_id(path)
            .ok_or(ComponentStoreError::ComponentNotFound)?;
        self.interact_dyn_mut(id, callback)
    }

    #[inline]
    pub fn interact_dyn_local_by_path<T: 'static>(
        &self,
        path: &ComponentPath,
        callback: impl FnOnce(&dyn Component) -> T,
    ) -> Result<T, ComponentStoreError> {
        let id = self
            .get_id(path)
            .ok_or(ComponentStoreError::ComponentNotFound)?;
        self.interact_dyn_local(id, callback)
    }

    #[inline]
    pub fn interact_dyn_local_mut_by_path<T: 'static>(
        &self,
        path: &ComponentPath,
        callback: impl FnOnce(&mut dyn Component) -> T,
    ) -> Result<T, ComponentStoreError> {
        let id = self
            .get_id(path)
            .ok_or(ComponentStoreError::ComponentNotFound)?;
        self.interact_dyn_local_mut(id, callback)
    }
}
