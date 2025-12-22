use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fmt::Debug,
    sync::Arc,
};

use rustc_hash::FxBuildHasher;

use crate::{
    component::{Component, ComponentHandle, TypedComponentHandle},
    machine::builder::SchedulerParticipation,
    path::{MultiemuPath, Namespace},
    scheduler::{EventManager, Period, PreemptionSignal},
};

struct ComponentInfo {
    component: ComponentHandle,
    type_id: TypeId,
}

impl Debug for ComponentInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComponentInfo").finish()
    }
}

#[derive(Debug, Default)]
/// The store for components
pub struct ComponentRegistry
// This absolutely has to be thread-safe
where
    Self: Send + Sync,
{
    components: HashMap<MultiemuPath, ComponentInfo, FxBuildHasher>,
}

impl ComponentRegistry {
    pub(crate) fn insert_component<C: Component>(
        &mut self,
        path: MultiemuPath,
        scheduler_participation: SchedulerParticipation,
        event_manager: Arc<EventManager>,
        interrupt: Arc<PreemptionSignal>,
        component: C,
    ) {
        assert!(path.namespace() == Namespace::Component);

        self.components.insert(
            path,
            ComponentInfo {
                component: ComponentHandle::new(
                    scheduler_participation,
                    event_manager,
                    interrupt,
                    component,
                ),
                type_id: TypeId::of::<C>(),
            },
        );
    }

    pub(crate) fn interact_all(&self, mut callback: impl FnMut(&MultiemuPath, &dyn Component)) {
        self.components.iter().for_each(|(path, info)| {
            info.component
                .interact_without_synchronization(|component| callback(path, component))
        });
    }

    pub(crate) fn interact_all_mut(
        &self,
        mut callback: impl FnMut(&MultiemuPath, &mut dyn Component),
    ) {
        self.components.iter().for_each(|(path, info)| {
            info.component
                .interact_mut_without_synchronization(|component| callback(path, component))
        });
    }

    #[inline]
    pub fn interact<C: Component, T>(
        &self,
        path: &MultiemuPath,
        current_timestamp: Period,
        callback: impl FnOnce(&C) -> T,
    ) -> Option<T> {
        self.interact_dyn(path, current_timestamp, |component| {
            let component = (component as &dyn Any).downcast_ref::<C>().unwrap();
            callback(component)
        })
    }

    #[inline]
    pub fn interact_mut<C: Component, T>(
        &self,
        path: &MultiemuPath,
        current_timestamp: Period,
        callback: impl FnOnce(&mut C) -> T,
    ) -> Option<T> {
        self.interact_dyn_mut(path, current_timestamp, |component| {
            let component = (component as &mut dyn Any).downcast_mut::<C>().unwrap();
            callback(component)
        })
    }

    #[inline]
    pub fn interact_dyn<T>(
        &self,
        path: &MultiemuPath,
        current_timestamp: Period,
        callback: impl FnOnce(&dyn Component) -> T,
    ) -> Option<T> {
        let component_info = if path.namespace() == Namespace::Resource {
            self.components.get(&path.parent()?)?
        } else {
            self.components.get(path)?
        };

        Some(
            component_info
                .component
                .interact(current_timestamp, |component| callback(component)),
        )
    }

    #[inline]
    pub fn interact_dyn_mut<T>(
        &self,
        path: &MultiemuPath,
        current_timestamp: Period,
        callback: impl FnOnce(&mut dyn Component) -> T,
    ) -> Option<T> {
        let component_info = if path.namespace() == Namespace::Resource {
            self.components.get(&path.parent()?)?
        } else {
            self.components.get(path)?
        };

        Some(
            component_info
                .component
                .interact_mut(current_timestamp, |component| callback(component)),
        )
    }

    pub fn typed_handle<C: Component>(
        &self,
        path: &MultiemuPath,
    ) -> Option<TypedComponentHandle<C>> {
        let component_info = self.components.get(path)?;

        assert_eq!(component_info.type_id, TypeId::of::<C>());

        Some(unsafe { TypedComponentHandle::new(component_info.component.clone()) })
    }

    pub fn handle(&self, path: &MultiemuPath) -> Option<ComponentHandle> {
        let component_info = self.components.get(path)?;

        Some(component_info.component.clone())
    }

    pub(crate) fn interact_without_synchronization<C: Component, T>(
        &self,
        path: &MultiemuPath,
        callback: impl FnOnce(&C) -> T,
    ) -> Option<T> {
        self.interact_dyn_without_synchronization(path, |component| {
            let component = (component as &dyn Any).downcast_ref::<C>().unwrap();
            callback(component)
        })
    }

    pub(crate) fn interact_mut_without_synchronization<C: Component, T>(
        &self,
        path: &MultiemuPath,
        callback: impl FnOnce(&mut C) -> T,
    ) -> Option<T> {
        self.interact_dyn_mut_without_synchronization(path, |component| {
            let component = (component as &mut dyn Any).downcast_mut::<C>().unwrap();
            callback(component)
        })
    }

    pub(crate) fn interact_dyn_without_synchronization<T>(
        &self,
        path: &MultiemuPath,
        callback: impl FnOnce(&dyn Component) -> T,
    ) -> Option<T> {
        let component_info = self.components.get(path)?;

        Some(
            component_info
                .component
                .interact_without_synchronization(|component| callback(component)),
        )
    }

    pub(crate) fn interact_dyn_mut_without_synchronization<T>(
        &self,
        path: &MultiemuPath,
        callback: impl FnOnce(&mut dyn Component) -> T,
    ) -> Option<T> {
        let component_info = self.components.get(path)?;

        Some(
            component_info
                .component
                .interact_mut_without_synchronization(|component| callback(component)),
        )
    }
}
