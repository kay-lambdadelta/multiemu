use super::{
    Component, ComponentId,
    store::{ComponentStore, Error},
};
use std::{
    any::{Any, TypeId},
    fmt::Debug,
    sync::Arc,
};

#[derive(Clone)]
enum ComponentLocation {
    Here(Arc<dyn Component + Send + Sync>),
    Elsewhere,
}

pub struct ComponentRef<C: Component> {
    location: ComponentLocation,
    store: Arc<ComponentStore>,
    component_id: ComponentId,
    _phantom: std::marker::PhantomData<C>,
}

impl<C: Component> Clone for ComponentRef<C> {
    fn clone(&self) -> Self {
        Self {
            location: self.location.clone(),
            store: self.store.clone(),
            component_id: self.component_id,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<C: Component + Debug> Debug for ComponentRef<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = String::default();

        self.interact(|component| {
            s.push_str(&format!("{:?}", component));
        })
        .unwrap();

        f.debug_struct("ComponentRef")
            .field("component", &s)
            .field("component_id", &self.component_id)
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
                component_id,
                _phantom: std::marker::PhantomData,
            }
        } else {
            Self {
                location: ComponentLocation::Elsewhere,
                store: component_store,
                component_id,
                _phantom: std::marker::PhantomData,
            }
        }
    }

    pub fn interact(&self, callback: impl FnOnce(&C) + Send) -> Result<(), Error> {
        match &self.location {
            ComponentLocation::Here(component) => {
                let component = (component.as_ref() as &dyn Any)
                    .downcast_ref::<C>()
                    .expect("Component type mismatch");

                callback(component);

                Ok(())
            }
            ComponentLocation::Elsewhere => self.store.interact::<C>(self.component_id, callback),
        }
    }
}
