use crate::GraphicsApi;
use core::ops::BitOr;
use nalgebra::DMatrix;
use palette::Srgba;

#[derive(Default, Debug)]
/// Marker trait for software rendering
///
/// This is the only graphics api that is guaranteed to always work anywhere
pub struct Software;

#[derive(Default, Clone, Debug)]
/// Does not actually require any extensions
pub struct Features;

impl BitOr for Features {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        rhs
    }
}

pub type InitializationData = ();

pub type FramebufferTexture = DMatrix<Srgba<u8>>;

impl GraphicsApi for Software {
    type InitializationData = InitializationData;
    type FramebufferTexture = FramebufferTexture;
    type Features = Features;
}
