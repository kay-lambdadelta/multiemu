use super::{Component, ComponentId, store::ComponentStore};
use std::{any::TypeId, fmt::Debug, sync::Arc};

#[derive(Clone)]
enum ComponentLocation {
    Here(Arc<dyn Component + Send + Sync>),
    Elsewhere(ComponentId),
}

pub struct ComponentRef<C: Component> {
    location: ComponentLocation,
    store: Arc<ComponentStore>,
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
        let mut s = String::default();

        self.interact(|component| {
            s.push_str(&format!("{:?}", component));
        });

        f.debug_struct("ComponentRef")
            .field("component", &s)
            .finish()
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
                let component = component
                    .as_any()
                    .downcast_ref::<C>()
                    .expect("Component type mismatch");

                callback(component);
            }
            ComponentLocation::Elsewhere(component_id) => {
                self.store.interact::<C>(*component_id, callback);
            }
        }
    }
}
