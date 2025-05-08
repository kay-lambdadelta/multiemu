use super::ComponentBuilder;
use crate::{component::Component, display::backend::RenderBackend};
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
};

#[allow(clippy::type_complexity)]
/// Holds the data that's needed per backend
pub struct BackendSpecificData<R: RenderBackend> {
    /// The preferred extensions for the context
    pub preferred_extensions: R::ContextExtensionSpecification,
    /// The required extensions for the context
    pub required_extensions: R::ContextExtensionSpecification,
    /// Callback for when display data is initialized per above specifications
    pub set_display_callback: Box<
        dyn FnOnce(
            &dyn Component,
            Arc<R::ComponentInitializationData>,
        ) -> Arc<R::ComponentFramebuffer>,
    >,
}

#[derive(Default)]
pub struct DisplayMetadata {
    pub backend_specific_data: HashMap<TypeId, Box<dyn Any>>,
}

impl<C: Component> ComponentBuilder<'_, C> {
    // Set your config for a given render backend
    pub fn set_display_config<R: RenderBackend>(
        mut self,
        preferred_extensions: Option<R::ContextExtensionSpecification>,
        required_extensions: Option<R::ContextExtensionSpecification>,
        set_display_callback: impl FnOnce(
            &C,
            Arc<R::ComponentInitializationData>,
        ) -> Arc<R::ComponentFramebuffer>
        + 'static,
    ) -> Self {
        let backend_specific_data = &mut self
            .component_metadata
            .display
            .get_or_insert_default()
            .backend_specific_data;

        backend_specific_data.insert(
            TypeId::of::<R>(),
            Box::new(BackendSpecificData::<R> {
                preferred_extensions: preferred_extensions.unwrap_or_default(),
                required_extensions: required_extensions.unwrap_or_default(),
                set_display_callback: Box::new(move |component, initialization_data| {
                    let component = (component as &dyn Any).downcast_ref::<C>().unwrap();
                    set_display_callback(component, initialization_data)
                }),
            }),
        );

        self
    }
}
