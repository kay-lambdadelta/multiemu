use crate::component::{Component, ComponentHandle, ComponentPath, ErasedComponentHandle};
use rustc_hash::FxBuildHasher;
use std::{
    any::{Any, TypeId},
    fmt::Debug,
};

#[derive(Debug)]
struct ComponentInfo {
    component: ErasedComponentHandle,
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
            callback(path, &*component_info.component.read());

            true
        });
    }

    pub(crate) fn interact_all_mut(
        &self,
        mut callback: impl FnMut(&ComponentPath, &mut dyn Component),
    ) {
        self.components.iter_sync(|path, component_info| {
            callback(path, &mut *component_info.component.write());

            true
        });
    }

    pub(crate) fn insert_component<C: Component>(&self, path: ComponentPath, component: C) {
        self.components
            .insert_sync(
                path,
                ComponentInfo {
                    component: ErasedComponentHandle::new(component),
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
            callback(&*component_info.component.read())
        })
    }

    #[inline]
    pub fn interact_dyn_mut<T: 'static>(
        &self,
        path: &ComponentPath,
        callback: impl FnOnce(&mut dyn Component) -> T,
    ) -> Option<T> {
        self.components.read_sync(path, |_, component_info| {
            callback(&mut *component_info.component.write())
        })
    }

    pub fn get_erased(&self, path: &ComponentPath) -> Option<ErasedComponentHandle> {
        self.components
            .read_sync(path, |_, component_info| component_info.component.clone())
    }

    pub fn get<C: Component>(&self, path: &ComponentPath) -> Option<ComponentHandle<C>> {
        self.components.read_sync(path, |_, component_info| {
            let component = component_info.component.clone();

            // Ensure the component actually matches the generic
            assert_eq!(component_info.type_id, TypeId::of::<C>());

            unsafe { ComponentHandle::new(component) }
        })
    }
}
