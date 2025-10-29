use crate::{
    component::Component,
    scheduler::{TaskData, TaskId},
};
use std::{
    any::{Any, TypeId},
    collections::BTreeMap,
    marker::PhantomData,
    num::NonZero,
    ops::DerefMut,
    sync::{Arc, RwLock},
};

// HACK: Add a generic so we can coerce this unsized
#[derive(Debug)]
struct HandleInner<T: ?Sized> {
    tasks: BTreeMap<TaskId, TaskData>,
    component: T,
}

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

    #[inline]
    pub fn interact<T>(&self, callback: impl FnOnce(&dyn Component) -> T) -> T {
        let guard = self.0.read().unwrap();

        let any_debt = guard.tasks.iter().any(|(_, data)| data.debt > 0);

        if any_debt {
            drop(guard);
            return self.interact_mut(|component| callback(component));
        }

        callback(&guard.component)
    }

    #[inline]
    pub fn interact_mut<T>(&self, callback: impl FnOnce(&mut dyn Component) -> T) -> T {
        let mut guard = self.0.write().unwrap();
        let guard = guard.deref_mut();

        for (_, data) in guard.tasks.iter_mut() {
            if let Some(debt) = NonZero::new(data.debt) {
                (data.callback)(&mut guard.component, debt);
                data.debt = 0;
            }
        }

        callback(&mut guard.component)
    }

    #[inline]
    /// Gets the component and its task without clearing debt
    pub(crate) fn interact_mut_with_task<T>(
        &self,
        task_id: TaskId,
        callback: impl FnOnce(&mut dyn Component, &mut TaskData) -> T,
    ) -> T {
        let mut guard = self.0.write().unwrap();
        let guard = guard.deref_mut();

        callback(&mut guard.component, guard.tasks.get_mut(&task_id).unwrap())
    }
}

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

    #[inline]
    pub fn interact<T: 'static>(&self, callback: impl FnOnce(&C) -> T) -> T {
        self.component.interact(|component| {
            debug_assert_eq!(TypeId::of::<C>(), (component as &dyn Any).type_id());
            let component = unsafe { &*(component as *const dyn Component as *const C) };

            callback(component)
        })
    }

    #[inline]
    pub fn interact_mut<T: 'static>(&self, callback: impl FnOnce(&mut C) -> T) -> T {
        self.component.interact_mut(|component| {
            debug_assert_eq!(TypeId::of::<C>(), (component as &dyn Any).type_id());
            let component = unsafe { &mut *(component as *mut dyn Component as *mut C) };

            callback(component)
        })
    }
}
