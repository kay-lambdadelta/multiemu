use crate::component::{Component, ComponentId, ComponentPath};
use rustc_hash::FxBuildHasher;
use std::{
    any::Any,
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
struct ComponentInfo {
    component: Arc<RwLock<dyn Component + Send + Sync>>,
    path: ComponentPath,
}

#[derive(Debug, Default)]
/// The store for components
pub struct ComponentRegistry
// This absolutely has to be thread-safe
where
    Self: Send + Sync,
{
    components: Vec<Option<ComponentInfo>>,
    component_ids: HashMap<ComponentPath, ComponentId, FxBuildHasher>,
}

impl ComponentRegistry {
    pub fn get_path(&self, component_id: ComponentId) -> ComponentPath {
        self.components[component_id.get() as usize]
            .as_ref()
            .unwrap()
            .path
            .clone()
    }

    pub fn get_id(&self, path: &ComponentPath) -> Option<ComponentId> {
        self.component_ids.get(path).copied()
    }

    pub(crate) fn interact_all(&self, mut callback: impl FnMut(&ComponentPath, &dyn Component)) {
        for component_info in self.components.iter() {
            let component_info = component_info.as_ref().unwrap();

            callback(
                &component_info.path,
                &*component_info.component.read().unwrap(),
            );
        }
    }

    pub(crate) fn interact_all_mut(
        &self,
        mut callback: impl FnMut(&ComponentPath, &mut dyn Component),
    ) {
        for component_info in self.components.iter() {
            let component_info = component_info.as_ref().unwrap();

            callback(
                &component_info.path,
                &mut *component_info.component.write().unwrap(),
            );
        }
    }

    pub(crate) fn reserve_component(&mut self, path: ComponentPath) {
        let index = self.components.len();
        self.components.push(None);

        let component_id = ComponentId::new(index.try_into().unwrap());
        self.component_ids.insert(path, component_id);
    }

    pub(crate) fn insert_component(&mut self, path: ComponentPath, component: impl Component) {
        let id = self.component_ids.get(&path).unwrap();
        self.components[id.get() as usize] = Some(ComponentInfo {
            component: Arc::new(RwLock::new(component)),
            path,
        });
    }

    // Interacts with a component wherever it may be
    #[inline]
    pub fn interact_dyn<T: 'static>(
        &self,
        component_id: ComponentId,
        callback: impl FnOnce(&dyn Component) -> T,
    ) -> Result<T, ComponentStoreError> {
        let component_info = self.components[component_id.get() as usize]
            .as_ref()
            .unwrap();

        Ok(callback(&*component_info.component.read().unwrap()))
    }

    #[inline]
    pub fn interact_dyn_mut<T: 'static>(
        &self,
        component_id: ComponentId,
        callback: impl FnOnce(&mut dyn Component) -> T,
    ) -> Result<T, ComponentStoreError> {
        let component_info = self.components[component_id.get() as usize]
            .as_ref()
            .unwrap();

        Ok(callback(&mut *component_info.component.write().unwrap()))
    }

    #[inline]
    pub fn interact<C: Component, T: 'static>(
        &self,
        component_id: ComponentId,
        callback: impl FnOnce(&C) -> T,
    ) -> Result<T, ComponentStoreError> {
        self.interact_dyn(component_id, |component| {
            let component = (component as &dyn Any).downcast_ref::<C>().unwrap();
            callback(component)
        })
    }

    #[inline]
    pub fn interact_mut<C: Component, T: 'static>(
        &self,
        component_id: ComponentId,
        callback: impl FnOnce(&mut C) -> T,
    ) -> Result<T, ComponentStoreError> {
        self.interact_dyn_mut(component_id, |component| {
            let component = (component as &mut dyn Any).downcast_mut::<C>().unwrap();
            callback(component)
        })
    }

    #[inline]
    pub fn interact_by_path<C: Component, T: 'static>(
        &self,
        path: &ComponentPath,
        callback: impl FnOnce(&C) -> T,
    ) -> Result<T, ComponentStoreError> {
        let id = self
            .get_id(path)
            .ok_or(ComponentStoreError::ComponentNotFound)?;
        self.interact(id, callback)
    }

    #[inline]
    pub fn interact_mut_by_path<C: Component, T: 'static>(
        &self,
        path: &ComponentPath,
        callback: impl FnOnce(&mut C) -> T,
    ) -> Result<T, ComponentStoreError> {
        let id = self
            .get_id(path)
            .ok_or(ComponentStoreError::ComponentNotFound)?;
        self.interact_mut(id, callback)
    }

    #[inline]
    pub fn interact_dyn_by_path<T: 'static>(
        &self,
        path: &ComponentPath,
        callback: impl FnOnce(&dyn Component) -> T,
    ) -> Result<T, ComponentStoreError> {
        let id = self
            .get_id(path)
            .ok_or(ComponentStoreError::ComponentNotFound)?;
        self.interact_dyn(id, callback)
    }

    #[inline]
    pub fn interact_dyn_mut_by_path<T: 'static>(
        &self,
        path: &ComponentPath,
        callback: impl FnOnce(&mut dyn Component) -> T,
    ) -> Result<T, ComponentStoreError> {
        let id = self
            .get_id(path)
            .ok_or(ComponentStoreError::ComponentNotFound)?;
        self.interact_dyn_mut(id, callback)
    }
}
