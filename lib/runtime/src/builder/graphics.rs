use crate::graphics::GraphicsCallback;
use multiemu_graphics::GraphicsApi;
use std::boxed::Box;

pub struct DisplayMetadata<R: GraphicsApi> {
    /// The preferred extensions for the context
    pub preferred_extensions: Option<R::ContextExtensionSpecification>,
    /// The required extensions for the context
    pub required_extensions: Option<R::ContextExtensionSpecification>,
    /// Callback for when display data is initialized per above specifications
    pub callback: Box<dyn GraphicsCallback<R>>,
}
