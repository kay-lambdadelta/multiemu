use crate::display::{ContextExtensionSpecification, RenderBackend};
use multiemu_config::graphics::GraphicsApi;
use nalgebra::DMatrix;
use palette::Srgba;

pub struct SoftwareRendering;

#[derive(Default, Clone)]
pub struct SoftwareContextExtentionSpecification;

impl ContextExtensionSpecification for SoftwareContextExtentionSpecification {
    fn combine(self, _other: Self) -> Self
    where
        Self: Sized,
    {
        Self
    }
}
pub type SoftwareComponentFramebuffer = DMatrix<Srgba<u8>>;

impl RenderBackend for SoftwareRendering {
    const GRAPHICS_API: GraphicsApi = GraphicsApi::Software;
    type ComponentInitializationData = SoftwareComponentInitializationData;
    type ComponentFramebuffer = SoftwareComponentFramebuffer;
    type ContextExtensionSpecification = SoftwareContextExtentionSpecification;
}

#[derive(Default)]
pub struct SoftwareComponentInitializationData;
