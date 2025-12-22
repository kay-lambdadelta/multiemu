use std::{
    any::{Any, TypeId},
    marker::PhantomData,
    sync::{Arc, RwLock},
};

use crate::{
    component::Component,
    machine::builder::SchedulerParticipation,
    scheduler::{EventManager, Period, PreemptionSignal, SynchronizationContext},
};

#[derive(Debug)]
struct SynchronizationData {
    /// Timestamp this component is actually updated to
    updated_timestamp: Period,
    /// Interrupt receiver
    interrupt: Arc<PreemptionSignal>,
}

// HACK: Add a generic so we can coerce this unsized
#[derive(Debug)]
struct HandleInner<T: ?Sized> {
    synchronization_data: Option<SynchronizationData>,
    component: T,
}

/// A handle and storage for a component, and the tasks associated with it
#[derive(Debug, Clone)]
pub struct ComponentHandle {
    inner: Arc<RwLock<HandleInner<dyn Component>>>,
    event_manager: Arc<EventManager>,
}

impl ComponentHandle {
    pub(crate) fn new(
        scheduler_participation: SchedulerParticipation,
        event_manager: Arc<EventManager>,
        interrupt: Arc<PreemptionSignal>,
        component: impl Component,
    ) -> Self {
        let synchronization_data = if matches!(
            scheduler_participation,
            SchedulerParticipation::OnDemand | SchedulerParticipation::SchedulerDriven
        ) {
            Some(SynchronizationData {
                updated_timestamp: Period::default(),
                interrupt,
            })
        } else {
            None
        };

        Self {
            inner: Arc::new(RwLock::new(HandleInner {
                component,
                synchronization_data,
            })),
            event_manager,
        }
    }

    /// Interact immutably with a component
    #[inline]
    pub fn interact<T>(
        &self,
        current_timestamp: Period,
        callback: impl FnOnce(&dyn Component) -> T,
    ) -> T {
        let guard = self.inner.read().unwrap();

        if let Some(SynchronizationData {
            updated_timestamp, ..
        }) = &guard.synchronization_data
        {
            let delta = current_timestamp - updated_timestamp;

            // Check if our current timestamp needs updating
            if guard.component.needs_work(delta) {
                drop(guard);
                return self.interact_mut(current_timestamp, |component| callback(component));
            }
        }

        callback(&guard.component)
    }

    /// Interact mutably with a component
    #[inline]
    pub fn interact_mut<T>(
        &self,
        current_timestamp: Period,
        callback: impl FnOnce(&mut dyn Component) -> T,
    ) -> T {
        let mut guard = self.inner.write().unwrap();
        let mut delta;
        let mut last_attempted_allocation = None;

        if guard.synchronization_data.is_some() {
            // Loop until the component is fully updated, processing events when relevant
            loop {
                let guard_inner = &mut *guard;
                let synchronization_data = guard_inner.synchronization_data.as_mut().unwrap();

                // Update delta in case something happened when we dropped and reacquired the lock
                delta = current_timestamp - synchronization_data.updated_timestamp;

                // Check if the component is done or there is no allocated time
                if delta == Period::ZERO || !guard_inner.component.needs_work(delta) {
                    break;
                }

                let context = SynchronizationContext {
                    event_manager: &self.event_manager,
                    updated_timestamp: &mut synchronization_data.updated_timestamp,
                    target_timestamp: current_timestamp,
                    last_attempted_allocation: &mut last_attempted_allocation,
                    interrupt: &synchronization_data.interrupt,
                };

                guard_inner.component.synchronize(context);

                // Prevent bad synchronization logic from spinning forever
                let last_attempted_allocation = last_attempted_allocation.take().expect(
                    "Synchronization attempt for component did not attempt to allocate time",
                );

                // Update delta
                delta = current_timestamp - synchronization_data.updated_timestamp;

                // If the component yielded and there is still work, check events and try to run it again
                if guard_inner.component.needs_work(delta) {
                    // Try to consume any events that blocked this time allocation
                    let timestamp =
                        synchronization_data.updated_timestamp + last_attempted_allocation;

                    // Drop the lock so that events that touch this component do not deadlock
                    drop(guard);

                    // consume our events
                    self.event_manager.consume_events(timestamp);

                    // Reacquire lock
                    guard = self.inner.write().unwrap();
                } else {
                    break;
                }
            }
        }

        callback(&mut guard.component)
    }

    pub(crate) fn interact_without_synchronization<T>(
        &self,
        callback: impl FnOnce(&dyn Component) -> T,
    ) -> T {
        let guard = self.inner.read().unwrap();

        callback(&guard.component)
    }

    pub(crate) fn interact_mut_without_synchronization<T>(
        &self,
        callback: impl FnOnce(&mut dyn Component) -> T,
    ) -> T {
        let mut guard = self.inner.write().unwrap();

        callback(&mut guard.component)
    }
}

/// Helper type that acts like a [`ComponentHandle`] but does downcasting
/// for you
#[derive(Debug)]
pub struct TypedComponentHandle<C: Component> {
    component: ComponentHandle,
    _phantom: PhantomData<C>,
}

impl<C: Component> Clone for TypedComponentHandle<C> {
    fn clone(&self) -> Self {
        Self {
            component: self.component.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<C: Component> TypedComponentHandle<C> {
    /// # SAFETY
    ///
    /// The component must match the type of the generic, this struct does not
    /// do type checking in release mode
    pub(crate) unsafe fn new(component: ComponentHandle) -> Self {
        Self {
            component,
            _phantom: PhantomData,
        }
    }

    /// Interact immutably with a component
    ///
    /// Note that this may or may not do an exclusively lock on the component.
    ///
    /// The chance of an exclusive access occuring is greatly increased if the
    /// component has lazy task
    #[inline]
    pub fn interact<T>(&self, current_timestamp: Period, callback: impl FnOnce(&C) -> T) -> T {
        self.component.interact(current_timestamp, |component| {
            debug_assert_eq!(TypeId::of::<C>(), (component as &dyn Any).type_id());
            let component = unsafe { &*std::ptr::from_ref::<dyn Component>(component).cast::<C>() };

            callback(component)
        })
    }

    /// Interact mutably with a component
    #[inline]
    pub fn interact_mut<T>(
        &self,
        current_timestamp: Period,
        callback: impl FnOnce(&mut C) -> T,
    ) -> T {
        self.component.interact_mut(current_timestamp, |component| {
            debug_assert_eq!(TypeId::of::<C>(), (component as &dyn Any).type_id());
            let component =
                unsafe { &mut *std::ptr::from_mut::<dyn Component>(component).cast::<C>() };

            callback(component)
        })
    }
}
