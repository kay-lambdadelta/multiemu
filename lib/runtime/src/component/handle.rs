use crate::component::Component;
use std::{
    any::Any,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

#[derive(Debug, Clone)]
pub struct ErasedComponentHandle {
    component: Arc<RwLock<dyn Component>>,
}

impl ErasedComponentHandle {
    pub(crate) fn new(component: impl Component) -> Self {
        Self {
            component: Arc::new(RwLock::new(component)),
        }
    }

    #[inline]
    pub fn read(&self) -> ErasedComponentHandleReadGuard<'_> {
        let guard = self.component.read().unwrap();

        ErasedComponentHandleReadGuard { component: guard }
    }

    #[inline]
    pub fn write(&self) -> ErasedComponentHandleWriteGuard<'_> {
        let guard = self.component.write().unwrap();

        ErasedComponentHandleWriteGuard { component: guard }
    }
}

pub struct ErasedComponentHandleReadGuard<'a> {
    component: RwLockReadGuard<'a, dyn Component>,
}

impl<'a> Deref for ErasedComponentHandleReadGuard<'a> {
    type Target = dyn Component;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.component.deref()
    }
}

pub struct ErasedComponentHandleWriteGuard<'a> {
    component: RwLockWriteGuard<'a, dyn Component>,
}

impl<'a> Deref for ErasedComponentHandleWriteGuard<'a> {
    type Target = dyn Component;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.component.deref()
    }
}

impl<'a> DerefMut for ErasedComponentHandleWriteGuard<'a> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.component.deref_mut()
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
    pub fn read(&self) -> ComponentHandleReadGuard<'_, C> {
        let guard = self.component.read();

        ComponentHandleReadGuard {
            component: guard,
            _phantom: PhantomData,
        }
    }

    #[inline]
    pub fn write(&self) -> ComponentHandleWriteGuard<'_, C> {
        let guard = self.component.write();

        ComponentHandleWriteGuard {
            component: guard,
            _phantom: PhantomData,
        }
    }
}

pub struct ComponentHandleReadGuard<'a, C> {
    component: ErasedComponentHandleReadGuard<'a>,
    _phantom: PhantomData<C>,
}

impl<'a, C: Component> Deref for ComponentHandleReadGuard<'a, C> {
    type Target = C;

    #[inline]
    fn deref(&self) -> &Self::Target {
        debug_assert!((self.component.deref() as &dyn Any).is::<C>());

        // The component must match the type of the generic
        unsafe { &*(self.component.deref() as &dyn Any as *const dyn Any as *const C) }
    }
}

pub struct ComponentHandleWriteGuard<'a, C> {
    component: ErasedComponentHandleWriteGuard<'a>,
    _phantom: PhantomData<C>,
}

impl<'a, C: Component> Deref for ComponentHandleWriteGuard<'a, C> {
    type Target = C;

    #[inline]
    fn deref(&self) -> &Self::Target {
        debug_assert!((self.component.deref() as &dyn Any).is::<C>());

        unsafe { &*(self.component.deref() as &dyn Any as *const dyn Any as *const C) }
    }
}

impl<'a, C: Component> DerefMut for ComponentHandleWriteGuard<'a, C> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        debug_assert!((self.component.deref() as &dyn Any).is::<C>());

        unsafe { &mut *(self.component.deref_mut() as &mut dyn Any as *mut dyn Any as *mut C) }
    }
}
