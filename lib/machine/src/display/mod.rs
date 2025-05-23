use backend::RenderApi;

pub mod backend;
pub mod shader;

pub struct RenderExtensions<R: RenderApi> {
    pub required: R::ContextExtensionSpecification,
    pub preferred: R::ContextExtensionSpecification,
}

impl<R: RenderApi> Default for RenderExtensions<R> {
    fn default() -> Self {
        Self {
            required: Default::default(),
            preferred: Default::default(),
        }
    }
}
