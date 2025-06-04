use super::{
    Component, ComponentId,
    store::{ComponentStore, Error},
};
use std::{
    any::{Any, TypeId},
    fmt::Debug,
    sync::{Arc, Weak},
};

#[derive(Clone)]
enum ComponentLocation {
    Here(Arc<dyn Component + Send + Sync>),
    Elsewhere(ComponentId),
}

pub struct ComponentRef<C: Component> {
    location: ComponentLocation,
    // Stop potential cycles
    store: Weak<ComponentStore>,
    _phantom: std::marker::PhantomData<C>,
}

impl<C: Component> Clone for ComponentRef<C> {
    fn clone(&self) -> Self {
        Self {
            location: self.location.clone(),
            store: self.store.clone(),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<C: Component + Debug> Debug for ComponentRef<C> {
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
    pub(super) fn new(
        component_id: ComponentId,
        component: Option<Arc<dyn Component + Send + Sync>>,
        component_store: Arc<ComponentStore>,
    ) -> Self {
        if let Some(component) = component {
            assert_eq!(
                TypeId::of::<C>(),
                component.as_ref().type_id(),
                "Component type mismatch"
            );

            Self {
                location: ComponentLocation::Here(component),
                store: Arc::downgrade(&component_store),
                _phantom: std::marker::PhantomData,
            }
        } else {
            Self {
                location: ComponentLocation::Elsewhere(component_id),
                store: Arc::downgrade(&component_store),
                _phantom: std::marker::PhantomData,
            }
        }
    }

    pub fn interact<T: Send + 'static>(
        &self,
        callback: impl FnOnce(&C) -> T + Send,
    ) -> Result<T, Error> {
        match &self.location {
            ComponentLocation::Here(component) => {
                let component = (component.as_ref() as &dyn Any)
                    .downcast_ref::<C>()
                    .expect("Component type mismatch");

                Ok(callback(component))
            }
            ComponentLocation::Elsewhere(component_id) => self
                .store
                .upgrade()
                .unwrap()
                .interact(*component_id, callback),
        }
    }

    pub fn interact_local<T: 'static>(&self, callback: impl FnOnce(&C) -> T) -> Result<T, Error> {
        match &self.location {
            ComponentLocation::Here(component) => {
                let component = (component.as_ref() as &dyn Any)
                    .downcast_ref::<C>()
                    .expect("Component type mismatch");

                Ok(callback(component))
            }
            ComponentLocation::Elsewhere(component_id) => self
                .store
                .upgrade()
                .unwrap()
                .interact_local(*component_id, callback),
        }
    }
}
