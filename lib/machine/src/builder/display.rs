use crossbeam::channel::Sender;

use super::ComponentBuilder;
use crate::{component::Component, display::RenderBackend};
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
};

#[allow(clippy::type_complexity)]
pub struct BackendSpecificData<R: RenderBackend> {
    pub preferred_extensions: R::ContextExtensionSpecification,
    pub required_extensions: R::ContextExtensionSpecification,
    pub set_display_callback: Box<
        dyn FnOnce(
                &dyn Component,
                Arc<R::ComponentInitializationData>,
                Sender<R::ComponentFramebuffer>,
            ) + Send,
    >,
}

#[derive(Default)]
pub struct DisplayMetadata {
    pub backend_specific_data: HashMap<TypeId, Box<dyn Any>>,
}

impl<C: Component> ComponentBuilder<'_, C> {
    pub fn set_display_config<R: RenderBackend>(
        mut self,
        preferred_extensions: Option<R::ContextExtensionSpecification>,
        required_extensions: Option<R::ContextExtensionSpecification>,
        set_display_callback: impl FnOnce(
            &C,
            Arc<R::ComponentInitializationData>,
            Sender<R::ComponentFramebuffer>,
        ) + Send
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
                set_display_callback: Box::new(
                    move |component, initialization_data, frame_channel| {
                        let component = component.as_any().downcast_ref::<C>().unwrap();
                        set_display_callback(component, initialization_data, frame_channel);
                    },
                ),
            }),
        );

        self
    }
}
