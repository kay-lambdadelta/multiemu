use super::{
    Component, ComponentId,
    registry::{ComponentRegistry, ComponentStoreError},
};
use std::{
    fmt::Debug,
    sync::{Arc, OnceLock},
};

pub struct ComponentRef<C: Component> {
    id: Arc<OnceLock<ComponentId>>,
    // I tested and this doesn't cause cycles
    registry: Arc<ComponentRegistry>,
    _phantom: std::marker::PhantomData<C>,
}

impl<C: Component> Clone for ComponentRef<C> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
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
    pub fn new(registry: Arc<ComponentRegistry>) -> Self {
        Self {
            id: Arc::new(OnceLock::new()),
            registry,
            _phantom: std::marker::PhantomData,
        }
    }

    pub(crate) fn set_id(&self, id: ComponentId) {
        self.id.set(id).unwrap();
    }

    /// Interacts with this component
    #[inline]
    pub fn interact<T: Send + 'static>(
        &self,
        callback: impl FnOnce(&C) -> T + Send,
    ) -> Result<T, ComponentStoreError> {
        self.registry
            .interact(*self.id.get().expect("Component not initialized"), callback)
    }

    /// Interacts with this component
    #[inline]
    pub fn interact_mut<T: Send + 'static>(
        &self,
        callback: impl FnOnce(&mut C) -> T + Send,
    ) -> Result<T, ComponentStoreError> {
        self.registry
            .interact_mut(*self.id.get().expect("Component not initialized"), callback)
    }

    /// Interacts with this component if its on the same (main) thread
    #[inline]

    pub fn interact_local<T: 'static>(
        &self,
        callback: impl FnOnce(&C) -> T,
    ) -> Result<T, ComponentStoreError> {
        self.registry
            .interact_local(*self.id.get().expect("Component not initialized"), callback)
    }

    /// Interacts with this component if its on the same (main) thread
    #[inline]
    pub fn interact_local_mut<T: 'static>(
        &self,
        callback: impl FnOnce(&mut C) -> T,
    ) -> Result<T, ComponentStoreError> {
        self.registry
            .interact_local_mut(*self.id.get().expect("Component not initialized"), callback)
    }

    #[inline]
    pub fn id(&self) -> ComponentId {
        *self.id.get().expect("Component not initialized")
    }
}
