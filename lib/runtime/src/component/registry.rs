use super::{Component, ComponentId};
use crate::{
    component::{ComponentPath, ComponentRef},
    scheduler::DebtClearer,
    utils::{Fragile, MainThreadQueue},
};
use nohash::BuildNoHashHasher;
use rustc_hash::FxBuildHasher;
use scc::ebr::Guard;
use std::{
    any::Any,
    boxed::Box,
    collections::HashSet,
    fmt::Debug,
    ops::{Deref, DerefMut},
    sync::{
        Arc, OnceLock, RwLock,
        atomic::{AtomicU16, Ordering},
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
enum ComponentStorage {
    Global(Box<dyn Component + Send + Sync>),
    // Use fragile to guard thread safety
    Local(Fragile<Box<dyn Component>>),
}

#[derive(Debug)]
struct ComponentInfo {
    storage: RwLock<ComponentStorage>,
    path: ComponentPath,
}

#[derive(Debug)]
/// The store for components
pub struct ComponentRegistry
// This absolutely has to be thread-safe
where
    Self: Send + Sync,
{
    components: scc::HashIndex<ComponentId, Arc<ComponentInfo>, BuildNoHashHasher<u16>>,
    component_ids: scc::HashMap<ComponentPath, ComponentId, FxBuildHasher>,
    main_thread_queue: Arc<MainThreadQueue>,
    current_component_id: AtomicU16,
    debt_clearer: OnceLock<DebtClearer>,
}

impl ComponentRegistry {
    pub fn new(main_thread_queue: Arc<MainThreadQueue>) -> Arc<Self> {
        Self {
            components: scc::HashIndex::default(),
            component_ids: scc::HashMap::default(),
            main_thread_queue,
            current_component_id: AtomicU16::new(1),
            debt_clearer: OnceLock::new(),
        }
        .into()
    }

    pub(crate) fn set_debt_clearer(&self, debt_clearer: DebtClearer) {
        self.debt_clearer.set(debt_clearer).unwrap();
    }

    pub fn get_path(&self, component_id: ComponentId) -> Option<ComponentPath> {
        self.components
            .get(&component_id)
            .map(|component_info| component_info.path.clone())
    }

    pub fn get_id(&self, path: &ComponentPath) -> Option<ComponentId> {
        self.component_ids.get(path).as_deref().copied()
    }

    pub(crate) fn interact_all(
        &self,
        mut callback: impl FnMut(&ComponentPath, &dyn Component) + Send,
    ) {
        let mut visited = HashSet::new();
        let guard = Guard::new();

        for (_, component_info) in self.components.iter(&guard) {
            // Scan can visit twice
            if visited.contains(&component_info.path) {
                return;
            }
            visited.insert(component_info.path.clone());

            match component_info.storage.read().unwrap().deref() {
                ComponentStorage::Global(component) => {
                    callback(&component_info.path, component.as_ref())
                }
                ComponentStorage::Local(component) => {
                    self.main_thread_queue.maybe_wait_on_main(|| {
                        callback(&component_info.path, component.get().unwrap().as_ref())
                    })
                }
            }
        }
    }

    pub(crate) fn interact_all_mut(
        &self,
        mut callback: impl FnMut(&ComponentPath, &mut dyn Component) + Send,
    ) {
        let mut visited = HashSet::new();
        let guard = Guard::new();

        for (_, component_info) in self.components.iter(&guard) {
            // Scan can visit twice
            if visited.contains(&component_info.path) {
                return;
            }
            visited.insert(component_info.path.clone());

            match component_info.storage.write().unwrap().deref_mut() {
                ComponentStorage::Global(component) => {
                    callback(&component_info.path, component.as_mut())
                }
                ComponentStorage::Local(component) => {
                    self.main_thread_queue.maybe_wait_on_main(|| {
                        callback(&component_info.path, component.get_mut().unwrap().as_mut())
                    })
                }
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

    pub fn insert_component_local(
        &self,
        path: ComponentPath,
        component_id: ComponentId,
        component: impl Component,
    ) {
        self.components
            .entry(component_id)
            .or_insert(Arc::new(ComponentInfo {
                storage: RwLock::new(ComponentStorage::Local(Fragile::new(Box::new(component)))),
                path,
            }));
    }

    pub(crate) fn insert_component(
        &self,
        path: ComponentPath,
        component_id: ComponentId,
        component: impl Component + Send + Sync,
    ) {
        self.components
            .entry(component_id)
            .or_insert(Arc::new(ComponentInfo {
                storage: RwLock::new(ComponentStorage::Global(Box::new(component))),
                path,
            }));
    }

    // Interacts with a component wherever it may be
    #[inline]
    pub fn interact_dyn<T: Send + 'static>(
        &self,
        component_id: ComponentId,
        callback: impl FnOnce(&dyn Component) -> T + Send,
    ) -> Result<T, ComponentStoreError> {
        // Make sure all cycle debts are cleared
        if let Some(debt_clearer) = self.debt_clearer.get() {
            debt_clearer.clear_debts(component_id);
        }
        let guard = Guard::new();

        match self
            .components
            .peek(&component_id, &guard)
            .ok_or(ComponentStoreError::ComponentNotFound)?
            .storage
            .read()
            .unwrap()
            .deref()
        {
            ComponentStorage::Local(component) => self
                .main_thread_queue
                .maybe_wait_on_main(|| Ok(callback(component.get().unwrap().as_ref()))),
            ComponentStorage::Global(component) => Ok(callback(component.as_ref())),
        }
    }

    #[inline]
    pub fn interact_dyn_mut<T: Send + 'static>(
        &self,
        component_id: ComponentId,
        callback: impl FnOnce(&mut dyn Component) -> T + Send,
    ) -> Result<T, ComponentStoreError> {
        if let Some(debt_clearer) = self.debt_clearer.get() {
            debt_clearer.clear_debts(component_id);
        }
        let guard = Guard::new();

        match self
            .components
            .peek(&component_id, &guard)
            .ok_or(ComponentStoreError::ComponentNotFound)?
            .storage
            .write()
            .unwrap()
            .deref_mut()
        {
            ComponentStorage::Local(component) => self
                .main_thread_queue
                .maybe_wait_on_main(|| Ok(callback(component.get_mut().unwrap().as_mut()))),
            ComponentStorage::Global(component) => Ok(callback(component.as_mut())),
        }
    }

    // Interacts with a component but restricted to the local instance, mostly for graphics initialization
    #[inline]
    pub fn interact_dyn_local<T: 'static>(
        &self,
        component_id: ComponentId,
        callback: impl FnOnce(&dyn Component) -> T,
    ) -> Result<T, ComponentStoreError> {
        // Make sure all cycle debts are cleared
        if let Some(debt_clearer) = self.debt_clearer.get() {
            debt_clearer.clear_debts(component_id);
        }

        match self
            .components
            .get(&component_id)
            .ok_or(ComponentStoreError::ComponentNotFound)?
            .storage
            .read()
            .unwrap()
            .deref()
        {
            ComponentStorage::Local(component) => Ok(callback(component.get().unwrap().as_ref())),
            ComponentStorage::Global(component) => Ok(callback(component.as_ref())),
        }
    }

    pub fn interact_dyn_local_mut<T: 'static>(
        &self,
        component_id: ComponentId,
        callback: impl FnOnce(&mut dyn Component) -> T,
    ) -> Result<T, ComponentStoreError> {
        if let Some(debt_clearer) = self.debt_clearer.get() {
            debt_clearer.clear_debts(component_id);
        }

        match self
            .components
            .get(&component_id)
            .ok_or(ComponentStoreError::ComponentNotFound)?
            .storage
            .write()
            .unwrap()
            .deref_mut()
        {
            ComponentStorage::Local(component) => {
                Ok(callback(component.get_mut().unwrap().as_mut()))
            }
            ComponentStorage::Global(component) => Ok(callback(component.as_mut())),
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
    pub fn interact_mut<C: Component, T: Send + 'static>(
        &self,
        component_id: ComponentId,
        callback: impl FnOnce(&mut C) -> T + Send,
    ) -> Result<T, ComponentStoreError> {
        self.interact_dyn_mut(component_id, |component| {
            let component = (component as &mut dyn Any).downcast_mut::<C>().unwrap();
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

    #[inline]
    pub fn interact_local_mut<C: Component, T: 'static>(
        &self,
        component_id: ComponentId,
        callback: impl FnOnce(&mut C) -> T,
    ) -> Result<T, ComponentStoreError> {
        self.interact_dyn_local_mut(component_id, |component| {
            let component = (component as &mut dyn Any).downcast_mut::<C>().unwrap();
            callback(component)
        })
    }

    pub fn get<C: Component>(self: &Arc<Self>, id: ComponentId) -> Option<ComponentRef<C>> {
        if self.components.contains(&id) {
            Some(ComponentRef::new(self.clone(), id))
        } else {
            None
        }
    }

    pub fn contains(&self, id: ComponentId) -> bool {
        self.components.contains(&id)
    }
}
