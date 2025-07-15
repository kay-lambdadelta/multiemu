use super::{Component, ComponentId};
use crate::{
    component::ComponentRef,
    scheduler::DebtClearer,
    utils::{Fragile, MainThreadQueue},
};
use multiemu_save::ComponentName;
use nohash::BuildNoHashHasher;
use std::{
    any::Any,
    boxed::Box,
    collections::HashMap,
    fmt::Debug,
    sync::{
        Arc, OnceLock, RwLock,
        atomic::{AtomicBool, AtomicU16, Ordering},
    },
};

#[derive(thiserror::Error, Debug)]
pub enum ComponentStoreError {
    #[error("Could not find component")]
    ComponentNotFound,
    #[error("Component could not be interact with due to its position")]
    ComponentUnreachable,
}

#[derive(Debug)]
enum ComponentLocation {
    Global(Box<dyn Component + Send + Sync>),
    // Use fragile to guard thread safety
    Local(Fragile<Box<dyn Component>>),
}

#[derive(Debug)]
struct ComponentInfo {
    location: ComponentLocation,
    name: ComponentName,
}

#[derive(Debug)]
/// The store for components
pub struct ComponentRegistry
// This absolutely has to be thread-safe
where
    Self: Send + Sync,
{
    components: RwLock<HashMap<ComponentId, ComponentInfo, BuildNoHashHasher<u16>>>,
    main_thread_queue: Arc<MainThreadQueue>,
    was_started: AtomicBool,
    current_component_id: AtomicU16,
    debt_clearer: OnceLock<DebtClearer>,
}

impl ComponentRegistry {
    pub fn new(main_thread_queue: Arc<MainThreadQueue>) -> Arc<Self> {
        Self {
            components: RwLock::default(),
            main_thread_queue,
            was_started: AtomicBool::new(false),
            current_component_id: AtomicU16::new(1),
            debt_clearer: OnceLock::new(),
        }
        .into()
    }

    pub(crate) fn set_debt_clearer(&self, debt_clearer: DebtClearer) {
        self.debt_clearer.set(debt_clearer).unwrap();
    }

    pub fn get_name(&self, component_id: ComponentId) -> Option<ComponentName> {
        self.components
            .read()
            .unwrap()
            .get(&component_id)
            .map(|component_info| component_info.name.clone())
    }

    pub(crate) fn interact_all(&self, mut callback: impl FnMut(&dyn Component) + Send) {
        if self.was_started.swap(true, Ordering::SeqCst) {
            panic!("Machine already started");
        }

        let component_location_guard = self.components.read().unwrap();

        for component_info in component_location_guard.values() {
            match &component_info.location {
                ComponentLocation::Global(component) => callback(component.as_ref()),
                ComponentLocation::Local(component) => self
                    .main_thread_queue
                    .maybe_wait_on_main(|| callback(component.get().unwrap().as_ref())),
            }
        }
    }

    pub fn generate_id(&self) -> ComponentId {
        ComponentId(
            self.current_component_id
                .fetch_add(1, Ordering::SeqCst)
                .try_into()
                .expect("Too many components"),
        )
    }

    pub fn insert_component(
        &self,
        name: ComponentName,
        component_id: ComponentId,
        component: impl Component,
    ) {
        let mut components_guard = self.components.write().unwrap();

        components_guard
            .entry(component_id)
            .or_insert(ComponentInfo {
                location: ComponentLocation::Local(Fragile::new(Box::new(component))),
                name,
            });
    }

    pub(crate) fn insert_component_global(
        &self,
        name: ComponentName,
        component_id: ComponentId,
        component: impl Component + Send + Sync,
    ) {
        let mut components_guard = self.components.write().unwrap();

        components_guard
            .entry(component_id)
            .or_insert(ComponentInfo {
                location: ComponentLocation::Global(Box::new(component)),
                name,
            });
    }

    // Interacts with a component wherever it may be
    #[inline]
    pub fn interact_dyn<T: Send + 'static>(
        &self,
        component_id: ComponentId,
        callback: impl FnOnce(&dyn Component) -> T + Send,
    ) -> Result<T, ComponentStoreError> {
        let component_location_guard = self.components.read().unwrap();

        // Make sure all cycle debts are cleared
        if let Some(debt_clearer) = self.debt_clearer.get() {
            debt_clearer.clear_debts(component_id);
        }

        match &component_location_guard
            .get(&component_id)
            .ok_or(ComponentStoreError::ComponentNotFound)?
            .location
        {
            ComponentLocation::Local(component) => Ok(self
                .main_thread_queue
                .maybe_wait_on_main(|| callback(component.get().unwrap().as_ref()))),
            ComponentLocation::Global(component) => Ok(callback(component.as_ref())),
        }
    }

    // Interacts with a component but restricted to the local instance, mostly for graphics initialization
    #[inline]
    pub fn interact_dyn_local<T: 'static>(
        &self,
        component_id: ComponentId,
        callback: impl FnOnce(&dyn Component) -> T,
    ) -> Result<T, ComponentStoreError> {
        let component_location_guard = self.components.read().unwrap();

        // Make sure all cycle debts are cleared
        if let Some(debt_clearer) = self.debt_clearer.get() {
            debt_clearer.clear_debts(component_id);
        }

        match &component_location_guard
            .get(&component_id)
            .ok_or(ComponentStoreError::ComponentNotFound)?
            .location
        {
            ComponentLocation::Local(component) => Ok(callback(component.get().unwrap().as_ref())),
            ComponentLocation::Global(component) => Ok(callback(component.as_ref())),
        }
    }

    #[inline]
    pub fn interact<C: Component, T: Send + 'static>(
        &self,
        component_id: ComponentId,
        callback: impl FnOnce(&C) -> T + Send,
    ) -> Result<T, ComponentStoreError> {
        self.interact_dyn(component_id, |component| {
            let component = (component as &dyn Any).downcast_ref::<C>().unwrap();
            callback(component)
        })
    }

    #[inline]
    pub fn interact_local<C: Component, T: 'static>(
        &self,
        component_id: ComponentId,
        callback: impl FnOnce(&C) -> T,
    ) -> Result<T, ComponentStoreError> {
        self.interact_dyn_local(component_id, |component| {
            let component = (component as &dyn Any).downcast_ref::<C>().unwrap();
            callback(component)
        })
    }

    pub fn get<C: Component>(self: &Arc<Self>, id: ComponentId) -> Option<ComponentRef<C>> {
        if self.components.read().unwrap().contains_key(&id) {
            Some(ComponentRef::new(self.clone(), id))
        } else {
            None
        }
    }
}
