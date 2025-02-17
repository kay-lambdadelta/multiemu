use super::{store::ComponentStore, Component, ComponentId};
use std::{any::TypeId, sync::Arc};

enum ComponentLocation {
    Here(Arc<dyn Component + Send + Sync>),
    Elsewhere(ComponentId),
}

pub struct ComponentRef<C: Component> {
    location: ComponentLocation,
    store: Arc<ComponentStore>,
    _phantom: std::marker::PhantomData<C>,
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
            assert!(
                TypeId::of::<C>() == component.as_ref().type_id(),
                "Component type mismatch"
            );

            Self {
                location: ComponentLocation::Here(component),
                store: component_store,
                _phantom: std::marker::PhantomData,
            }
        } else {
            Self {
                location: ComponentLocation::Elsewhere(component_id),
                store: component_store,
                _phantom: std::marker::PhantomData,
            }
        }
    }

    pub fn interact(&self, callback: impl FnOnce(&C) + Send) {
        match &self.location {
            ComponentLocation::Here(component) => {
                // FIXME: I would like to skip the downcast but i dunno if its possible
                let component = component.as_any().downcast_ref::<C>().unwrap();
                callback(component);
            }
            ComponentLocation::Elsewhere(component_id) => {
                self.store.interact::<C>(*component_id, callback);
            }
        }
    }
}
