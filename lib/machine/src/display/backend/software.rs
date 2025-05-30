use super::ContextExtensionSpecification;
use crate::display::RenderApi;
use multiemu_config::graphics::GraphicsApi;
use nalgebra::DMatrix;
use palette::Srgba;

/// Marker trait for software rendering, this should be the one used in tests and as a fallback
#[derive(Default, Debug)]
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

impl RenderApi for SoftwareRendering {
    const GRAPHICS_API: GraphicsApi = GraphicsApi::Software;
    type ComponentInitializationData = SoftwareComponentInitializationData;
    type ComponentFramebufferInner = DMatrix<Srgba<u8>>;
    type ContextExtensionSpecification = SoftwareContextExtentionSpecification;
}

#[derive(Default, Debug)]
pub struct SoftwareComponentInitializationData;
