use super::ContextExtensionSpecification;
use crate::display::RenderApi;
use nalgebra::DMatrix;
use palette::Srgba;

/// Marker trait for software rendering, this should be the one used in tests and as a fallback
#[derive(Default, Debug)]
pub struct SoftwareRendering;

#[derive(Default, Clone)]
pub struct SoftwareContextExtensionSpecification;

impl ContextExtensionSpecification for SoftwareContextExtensionSpecification {
    fn combine(self, _other: Self) -> Self
    where
        Self: Sized,
    {
        Self
    }
}

impl RenderApi for SoftwareRendering {
    type ComponentInitializationData = SoftwareComponentInitializationData;
    type ComponentFramebufferInner = DMatrix<Srgba<u8>>;
    type ContextExtensionSpecification = SoftwareContextExtensionSpecification;
}

#[derive(Default, Debug)]
pub struct SoftwareComponentInitializationData;
