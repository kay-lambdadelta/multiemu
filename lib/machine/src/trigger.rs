use crate::component::Component;
use std::sync::Mutex;

pub trait Trigger<C: Component>: Send + 'static {
    fn trigger(&mut self, component: &C);
}

impl<C: Component, F: FnMut(&C) + Send + 'static> Trigger<C> for F {
    fn trigger(&mut self, component: &C) {
        self(component);
    }
}

pub struct TriggerInfo<C: Component> {
    pub trigger: Mutex<Box<dyn Trigger<C>>>,
}

pub struct TriggerStore {}
