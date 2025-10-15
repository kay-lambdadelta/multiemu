use rustc_hash::FxBuildHasher;

use crate::component::{Component, ComponentHandle, ComponentPath};
use std::{
    any::{Any, TypeId},
    fmt::Debug,
    sync::{Arc, RwLock},
};

#[derive(Debug)]
struct ComponentInfo {
    component: Arc<RwLock<dyn Component>>,
    type_id: TypeId,
}

#[derive(Debug, Default)]
/// The store for components
pub struct ComponentRegistry
// This absolutely has to be thread-safe
where
    Self: Send + Sync,
{
    components: scc::HashMap<ComponentPath, ComponentInfo, FxBuildHasher>,
}

impl ComponentRegistry {
    pub(crate) fn interact_all(&self, mut callback: impl FnMut(&ComponentPath, &dyn Component)) {
        self.components.iter_sync(|path, component_info| {
            callback(path, &*component_info.component.read().unwrap());

            true
        });
    }

    pub(crate) fn interact_all_mut(
        &self,
        mut callback: impl FnMut(&ComponentPath, &mut dyn Component),
    ) {
        self.components.iter_sync(|path, component_info| {
            callback(path, &mut *component_info.component.write().unwrap());

            true
        });
    }

    pub(crate) fn insert_component<C: Component>(&self, path: ComponentPath, component: C) {
        self.components
            .insert_sync(
                path,
                ComponentInfo {
                    component: Arc::new(RwLock::new(component)),
                    type_id: TypeId::of::<C>(),
                },
            )
            .unwrap();
    }

    #[inline]
    pub fn interact<C: Component, T: 'static>(
        &self,
        path: &ComponentPath,
        callback: impl FnOnce(&C) -> T,
    ) -> Option<T> {
        self.interact_dyn(path, |component| {
            let component = (component as &dyn Any).downcast_ref::<C>().unwrap();
            callback(component)
        })
    }

    #[inline]
    pub fn interact_mut<C: Component, T: 'static>(
        &self,
        path: &ComponentPath,
        callback: impl FnOnce(&mut C) -> T,
    ) -> Option<T> {
        self.interact_dyn_mut(path, |component| {
            let component = (component as &mut dyn Any).downcast_mut::<C>().unwrap();
            callback(component)
        })
    }

    #[inline]
    pub fn interact_dyn<T: 'static>(
        &self,
        path: &ComponentPath,
        callback: impl FnOnce(&dyn Component) -> T,
    ) -> Option<T> {
        self.components.read_sync(path, |_, component_info| {
            callback(&*component_info.component.read().unwrap())
        })
    }

    #[inline]
    pub fn interact_dyn_mut<T: 'static>(
        &self,
        path: &ComponentPath,
        callback: impl FnOnce(&mut dyn Component) -> T,
    ) -> Option<T> {
        self.components.read_sync(path, |_, component_info| {
            callback(&mut *component_info.component.write().unwrap())
        })
    }

    pub fn get_direct(&self, path: &ComponentPath) -> Option<Arc<RwLock<dyn Component>>> {
        self.components
            .read_sync(path, |_, component_info| component_info.component.clone())
    }

    pub fn get<C: Component>(&self, path: &ComponentPath) -> Option<ComponentHandle<C>> {
        self.components.read_sync(path, |_, component_info| {
            let component = component_info.component.clone();
            assert_eq!(component_info.type_id, TypeId::of::<C>());
            ComponentHandle::new(component)
        })
    }
}
