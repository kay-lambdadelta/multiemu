use super::ComponentBuilder;
use crate::{
    component::Component,
    display::backend::{ComponentFramebuffer, RenderApi},
};
use std::{any::Any, boxed::Box};

pub trait DisplayCallback<R: RenderApi, C: Component>: 'static {
    fn get_framebuffer(self, component: &C) -> ComponentFramebuffer<R>;
}

impl<R: RenderApi, C: Component, F: FnOnce(&C) -> ComponentFramebuffer<R> + 'static>
    DisplayCallback<R, C> for F
{
    fn get_framebuffer(self, component: &C) -> ComponentFramebuffer<R> {
        self(component)
    }
}

pub struct DisplayMetadata<R: RenderApi> {
    /// The preferred extensions for the context
    pub preferred_extensions: Option<R::ContextExtensionSpecification>,
    /// The required extensions for the context
    pub required_extensions: Option<R::ContextExtensionSpecification>,
    /// Callback for when display data is initialized per above specifications
    #[allow(clippy::type_complexity)]
    pub set_display_callback: Box<dyn FnOnce(&dyn Component) -> ComponentFramebuffer<R>>,
}

impl<R: RenderApi, C: Component> ComponentBuilder<'_, R, C> {
    // Set your config for a given render backend
    pub fn set_display_config(
        mut self,
        preferred_extensions: Option<R::ContextExtensionSpecification>,
        required_extensions: Option<R::ContextExtensionSpecification>,
        set_display_callback: impl DisplayCallback<R, C>,
    ) -> Self {
        self.component_metadata.display = Some(DisplayMetadata {
            preferred_extensions,
            required_extensions,
            set_display_callback: Box::new(|component| {
                let component = (component as &dyn Any).downcast_ref::<C>().unwrap();
                set_display_callback.get_framebuffer(component)
            }),
        });

        self
    }
}
