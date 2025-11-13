use crate::{
    component::Component,
    scheduler::{TaskData, TaskId},
};
use std::{
    any::{Any, TypeId},
    collections::BTreeMap,
    marker::PhantomData,
    sync::{Arc, RwLock},
};

// HACK: Add a generic so we can coerce this unsized
#[derive(Debug)]
struct HandleInner<T: ?Sized> {
    tasks: BTreeMap<TaskId, TaskData>,
    component: T,
}

/// A handle and storage for a component, and the tasks associated with it
#[derive(Debug, Clone)]
pub struct ErasedComponentHandle(Arc<RwLock<HandleInner<dyn Component>>>);

impl ErasedComponentHandle {
    pub(crate) fn new(
        component: impl Component,
        tasks: impl IntoIterator<Item = (TaskId, TaskData)>,
    ) -> Self {
        Self(Arc::new(RwLock::new(HandleInner {
            tasks: tasks.into_iter().collect(),
            component,
        })))
    }

    /// Interact immutably with a component
    #[inline]
    pub fn interact<T>(&self, callback: impl FnOnce(&dyn Component) -> T) -> T {
        let guard = self.0.read().unwrap();

        callback(&guard.component)
    }

    /// Interact mutably with a component
    #[inline]
    pub fn interact_mut<T>(&self, callback: impl FnOnce(&mut dyn Component) -> T) -> T {
        let mut guard = self.0.write().unwrap();

        callback(&mut guard.component)
    }

    /// Gets the component and its task without clearing debt
    #[inline]
    pub(crate) fn interact_mut_with_task<T>(
        &self,
        task_id: TaskId,
        callback: impl FnOnce(&mut dyn Component, &mut TaskData) -> T,
    ) -> T {
        let mut guard = self.0.write().unwrap();
        let guard = &mut *guard;

        callback(&mut guard.component, guard.tasks.get_mut(&task_id).unwrap())
    }
}

/// Helper type that acts like a [`ErasedComponentHandle`] but does downcasting for you
#[derive(Debug)]
pub struct ComponentHandle<C: Component> {
    component: ErasedComponentHandle,
    _phantom: PhantomData<C>,
}

impl<C: Component> Clone for ComponentHandle<C> {
    fn clone(&self) -> Self {
        Self {
            component: self.component.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<C: Component> ComponentHandle<C> {
    /// # SAFETY
    ///
    /// The component must match the type of the generic, this struct does not do type checking in release mode
    pub(crate) unsafe fn new(component: ErasedComponentHandle) -> Self {
        Self {
            component,
            _phantom: PhantomData,
        }
    }

    /// Interact immutably with a component
    ///
    /// Note that this may or may not do an exclusively lock on the component.
    ///
    /// The chance of an exclusive access occuring is greatly increased if the component has lazy task
    #[inline]
    pub fn interact<T>(&self, callback: impl FnOnce(&C) -> T) -> T {
        self.component.interact(|component| {
            debug_assert_eq!(TypeId::of::<C>(), (component as &dyn Any).type_id());
            let component = unsafe { &*std::ptr::from_ref::<dyn Component>(component).cast::<C>() };

            callback(component)
        })
    }

    /// Interact mutably with a component
    #[inline]
    pub fn interact_mut<T>(&self, callback: impl FnOnce(&mut C) -> T) -> T {
        self.component.interact_mut(|component| {
            debug_assert_eq!(TypeId::of::<C>(), (component as &dyn Any).type_id());
            let component =
                unsafe { &mut *std::ptr::from_mut::<dyn Component>(component).cast::<C>() };

            callback(component)
        })
    }
}
