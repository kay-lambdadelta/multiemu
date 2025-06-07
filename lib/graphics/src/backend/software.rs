use super::ContextExtensionSpecification;
use crate::GraphicsApi;
use nalgebra::DMatrix;
use palette::Srgba;

#[derive(Default, Debug)]
/// Marker trait for software rendering
///
/// This is the only graphics api that is guaranteed to always work anywhere
pub struct Software;

#[derive(Default, Clone, Debug)]
/// Does not actually require any extensions
pub struct SoftwareContextExtensionSpecification;

impl ContextExtensionSpecification for SoftwareContextExtensionSpecification {
    fn combine(self, _other: Self) -> Self
    where
        Self: Sized,
    {
        Self
    }
}

impl GraphicsApi for Software {
    type ComponentGraphicsInitializationData = SoftwareComponentInitializationData;
    type ComponentFramebuffer = DMatrix<Srgba<u8>>;
    type ContextExtensionSpecification = SoftwareContextExtensionSpecification;
}

#[derive(Default, Debug)]
/// Does not require any initialization data
pub struct SoftwareComponentInitializationData;
