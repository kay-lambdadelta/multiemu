use super::{
    Component, ComponentId,
    store::{ComponentStore, ComponentStoreError},
};
use std::{
    fmt::Debug,
    sync::{Arc, Weak},
};

pub struct ComponentRef<C: Component> {
    id: ComponentId,
    // Stop potential cycles
    store: Weak<ComponentStore>,
    _phantom: std::marker::PhantomData<C>,
}

impl<C: Component> Clone for ComponentRef<C> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            store: self.store.clone(),
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
    pub(crate) fn new(component_store: Arc<ComponentStore>, component_id: ComponentId) -> Self {
        Self {
            id: component_id,
            store: Arc::downgrade(&component_store),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Interacts with this component
    #[inline]
    pub fn interact<T: Send + 'static>(
        &self,
        callback: impl FnOnce(&C) -> T + Send,
    ) -> Result<T, ComponentStoreError> {
        self.store.upgrade().unwrap().interact(self.id, callback)
    }

    /// Interacts with this component if its on the same (main) thread
    #[inline]

    pub fn interact_local<T: 'static>(
        &self,
        callback: impl FnOnce(&C) -> T,
    ) -> Result<T, ComponentStoreError> {
        self.store
            .upgrade()
            .unwrap()
            .interact_local(self.id, callback)
    }

    #[inline]
    pub fn id(&self) -> ComponentId {
        self.id
    }
}
