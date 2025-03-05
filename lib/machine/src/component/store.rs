use super::{Component, ComponentId, component_ref::ComponentRef};
use fxhash::FxBuildHasher;
use std::{
    borrow::Cow,
    cell::{LazyCell, RefCell},
    collections::HashMap,
    fmt::Debug,
    sync::Arc,
};

enum ComponentLocation {
    MainThread,
    Global(Arc<dyn Component + Send + Sync>),
}

impl Debug for ComponentLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComponentLocation::MainThread => f.write_str("MainThread"),
            ComponentLocation::Global(_) => f.write_str("Global"),
        }
    }
}

thread_local! {
    static IS_MAIN_THREAD: RefCell<bool> = const { RefCell::new(false) };
    static LOCAL_COMPONENT_STORE: LazyCell<RefCell<HashMap<ComponentId, Arc<dyn Component>>>> = const { LazyCell::new(RefCell::default) };
}

#[derive(Debug)]
pub struct ComponentStore
// This absolutely has to be thread-safe
where
    Self: Send + Sync,
{
    component_ids: scc::HashMap<Cow<'static, str>, ComponentId, FxBuildHasher>,
    component_location: scc::HashMap<ComponentId, ComponentLocation, FxBuildHasher>,
}

impl ComponentStore {
    pub(crate) fn insert_component(
        &self,
        manifest_name: &'static str,
        component_id: ComponentId,
        component: impl Component,
    ) {
        IS_MAIN_THREAD.with(|is_main_thread| {
            assert!(*is_main_thread.borrow());
        });

        LOCAL_COMPONENT_STORE.with(|task_component_store| {
            let mut task_component_store = task_component_store.borrow_mut();
            task_component_store.insert(component_id, Arc::new(component));
        });

        self.component_ids
            .insert(Cow::Borrowed(manifest_name), component_id)
            .unwrap();

        self.component_location
            .insert(component_id, ComponentLocation::MainThread)
            .unwrap();
    }

    pub(crate) fn insert_component_global(
        &self,
        manifest_name: &'static str,
        component_id: ComponentId,
        component: impl Component + Send + Sync,
    ) {
        IS_MAIN_THREAD.with(|is_main_thread| {
            assert!(*is_main_thread.borrow());
        });

        self.component_ids
            .insert(Cow::Borrowed(manifest_name), component_id)
            .unwrap();

        self.component_location
            .insert(component_id, ComponentLocation::Global(Arc::new(component)))
            .unwrap();
    }

    #[inline]
    // Interacts with a component wherever it may be
    pub(crate) fn interact_dyn(
        &self,
        component_id: ComponentId,
        callback: impl FnOnce(&dyn Component) + Send,
    ) {
        tracing::trace!("Interacting with component {:?}", component_id);
        let is_main_thread = IS_MAIN_THREAD.with(|is_main_thread| *is_main_thread.borrow());

        self.component_location
            .read(&component_id, |_, location| match location {
                ComponentLocation::MainThread => {
                    if is_main_thread {
                        LOCAL_COMPONENT_STORE.with(|thread_component_store| {
                            let thread_component_store = thread_component_store.borrow();
                            let component = thread_component_store.get(&component_id).unwrap();
                            callback(component.as_ref());
                        });
                    } else {
                        unimplemented!()
                    }
                }
                ComponentLocation::Global(component) => {
                    callback(component.as_ref());
                }
            })
            .expect("Could not locate component");
    }

    #[inline]
    // Interacts with a component but restricted to the local instance, mostly for graphics initialization
    pub(crate) fn interact_dyn_local(
        &self,
        component_id: ComponentId,
        callback: impl FnOnce(&dyn Component),
    ) {
        tracing::trace!("Interacting with component {:?}", component_id);
        let is_main_thread = IS_MAIN_THREAD.with(|is_main_thread| *is_main_thread.borrow());

        self.component_location
            .read(&component_id, |_, location| match location {
                ComponentLocation::MainThread => {
                    if is_main_thread {
                        LOCAL_COMPONENT_STORE.with(|thread_component_store| {
                            let thread_component_store = thread_component_store.borrow();
                            let component = thread_component_store.get(&component_id).unwrap();
                            callback(component.as_ref());
                        });
                    } else {
                        panic!("Could not interact with component")
                    }
                }
                ComponentLocation::Global(component) => {
                    callback(component.as_ref());
                }
            })
            .expect("Could not locate component");
    }

    #[inline]
    pub(crate) fn interact<C: Component>(
        &self,
        component_id: ComponentId,
        callback: impl FnOnce(&C) + Send,
    ) {
        self.interact_dyn(component_id, |component| {
            let component = component.as_any().downcast_ref::<C>().unwrap();
            callback(component);
        });
    }

    #[inline]
    pub(crate) fn interact_local<C: Component>(
        &self,
        component_id: ComponentId,
        callback: impl FnOnce(&C),
    ) {
        self.interact_dyn_local(component_id, |component| {
            let component = component.as_any().downcast_ref::<C>().unwrap();
            callback(component);
        });
    }

    #[inline]
    pub fn interact_by_name<C: Component>(
        &self,
        manifest_name: &str,
        callback: impl FnOnce(&C) + Send,
    ) {
        let component_id = *self.component_ids.get(manifest_name).unwrap().get();

        self.interact(component_id, callback);
    }

    #[inline]
    pub fn interact_by_name_local<C: Component>(
        &self,
        manifest_name: &str,
        callback: impl FnOnce(&C),
    ) {
        let component_id = *self.component_ids.get(manifest_name).unwrap().get();

        self.interact_local(component_id, callback);
    }

    pub fn get<C: Component>(self: &Arc<Self>, manifest_name: &str) -> Option<ComponentRef<C>> {
        let component_id = *self.component_ids.get(manifest_name).unwrap().get();

        let component = if let ComponentLocation::Global(component) =
            self.component_location.get(&component_id)?.get()
        {
            Some(component.clone())
        } else {
            None
        };

        Some(ComponentRef::new(component_id, component, self.clone()))
    }
}

impl Default for ComponentStore {
    fn default() -> Self {
        IS_MAIN_THREAD.with(|is_main_thread| {
            *is_main_thread.borrow_mut() = true;
        });

        LOCAL_COMPONENT_STORE.with(|task_component_store| {
            let mut task_component_store = task_component_store.borrow_mut();
            task_component_store.clear();
        });

        Self {
            component_ids: scc::HashMap::default(),
            component_location: scc::HashMap::default(),
        }
    }
}
