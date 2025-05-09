use super::ContextExtensionSpecification;
use crate::display::RenderBackend;
use multiemu_config::graphics::GraphicsApi;
use nalgebra::DMatrix;
use palette::Srgba;
use std::cell::RefCell;

/// Marker trait for software rendering, this should be the one used in tests and as a fallback
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

pub type SoftwareComponentFramebuffer = RefCell<DMatrix<Srgba<u8>>>;

impl RenderBackend for SoftwareRendering {
    const GRAPHICS_API: GraphicsApi = GraphicsApi::Software;
    type ComponentInitializationData = SoftwareComponentInitializationData;
    type ComponentFramebuffer = SoftwareComponentFramebuffer;
    type ContextExtensionSpecification = SoftwareContextExtentionSpecification;
}

#[derive(Default)]
pub struct SoftwareComponentInitializationData;
