use super::{
    Component, ComponentId,
    registry::{ComponentRegistry, ComponentStoreError},
};
use std::{
    fmt::Debug,
    sync::{Arc, Weak},
};

pub struct ComponentRef<C: Component> {
    id: ComponentId,
    // Stop potential cycles
    registry: Weak<ComponentRegistry>,
    _phantom: std::marker::PhantomData<C>,
}

impl<C: Component> Clone for ComponentRef<C> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            registry: self.registry.clone(),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<C: Component> Debug for ComponentRef<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug_struct = f.debug_struct("ComponentRef");

        // TODO: We need a better way to go about this
        self.interact_local(|component| {
            debug_struct.field("component", &component);
        })
        .unwrap();

        debug_struct.finish()
    }
}

/// SAFETY: This struct is perfectly safe to send between threads
unsafe impl<C: Component> Send for ComponentRef<C> {}
unsafe impl<C: Component> Sync for ComponentRef<C> {}

impl<C: Component> ComponentRef<C> {
    pub(crate) fn new(component_store: Arc<ComponentRegistry>, component_id: ComponentId) -> Self {
        Self {
            id: component_id,
            registry: Arc::downgrade(&component_store),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Interacts with this component
    #[inline]
    pub fn interact<T: Send + 'static>(
        &self,
        callback: impl FnOnce(&C) -> T + Send,
    ) -> Result<T, ComponentStoreError> {
        self.registry.upgrade().unwrap().interact(self.id, callback)
    }

    /// Interacts with this component
    #[inline]
    pub fn interact_mut<T: Send + 'static>(
        &self,
        callback: impl FnOnce(&mut C) -> T + Send,
    ) -> Result<T, ComponentStoreError> {
        self.registry
            .upgrade()
            .unwrap()
            .interact_mut(self.id, callback)
    }

    /// Interacts with this component if its on the same (main) thread
    #[inline]

    pub fn interact_local<T: 'static>(
        &self,
        callback: impl FnOnce(&C) -> T,
    ) -> Result<T, ComponentStoreError> {
        self.registry
            .upgrade()
            .unwrap()
            .interact_local(self.id, callback)
    }

    /// Interacts with this component if its on the same (main) thread
    #[inline]
    pub fn interact_local_mut<T: 'static>(
        &self,
        callback: impl FnOnce(&mut C) -> T,
    ) -> Result<T, ComponentStoreError> {
        self.registry
            .upgrade()
            .unwrap()
            .interact_local_mut(self.id, callback)
    }

    #[inline]
    pub fn id(&self) -> ComponentId {
        self.id
    }
}
