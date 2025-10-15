use crate::component::Component;
use std::{
    any::Any,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

pub struct ComponentHandleReadGuard<'a, C> {
    component: RwLockReadGuard<'a, dyn Component>,
    _phantom: PhantomData<C>,
}

impl<'a, C: Component> Deref for ComponentHandleReadGuard<'a, C> {
    type Target = C;

    fn deref(&self) -> &Self::Target {
        debug_assert!((self.component.deref() as &dyn Any).is::<C>());

        // The component must match the type of the generic
        unsafe { &*(self.component.deref() as &dyn Any as *const dyn Any as *const C) }
    }
}

pub struct ComponentHandleWriteGuard<'a, C> {
    component: RwLockWriteGuard<'a, dyn Component>,
    _phantom: PhantomData<C>,
}

impl<'a, C: Component> Deref for ComponentHandleWriteGuard<'a, C> {
    type Target = C;

    fn deref(&self) -> &Self::Target {
        debug_assert!((self.component.deref() as &dyn Any).is::<C>());

        unsafe { &*(self.component.deref() as &dyn Any as *const dyn Any as *const C) }
    }
}

impl<'a, C: Component> DerefMut for ComponentHandleWriteGuard<'a, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        debug_assert!((self.component.deref() as &dyn Any).is::<C>());

        unsafe { &mut *(self.component.deref_mut() as &mut dyn Any as *mut dyn Any as *mut C) }
    }
}

pub struct ComponentHandle<C: Component> {
    component: Arc<RwLock<dyn Component>>,
    _phantom: PhantomData<C>,
}

impl<C: Component> ComponentHandle<C> {
    /// # SAFETY
    ///
    /// The component must match the type of the generic, this struct does not do type checking in release mode
    pub(crate) unsafe fn new(component: Arc<RwLock<dyn Component>>) -> Self {
        Self {
            component,
            _phantom: PhantomData,
        }
    }

    pub fn read(&self) -> ComponentHandleReadGuard<'_, C> {
        let guard = self.component.read().unwrap();

        ComponentHandleReadGuard {
            component: guard,
            _phantom: PhantomData,
        }
    }

    pub fn write(&self) -> ComponentHandleWriteGuard<'_, C> {
        let guard = self.component.write().unwrap();

        ComponentHandleWriteGuard {
            component: guard,
            _phantom: PhantomData,
        }
    }
}
