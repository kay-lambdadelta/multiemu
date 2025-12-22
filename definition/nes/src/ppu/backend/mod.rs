use std::fmt::Debug;

use fluxemu_runtime::graphics::GraphicsApi;
use nalgebra::DMatrixViewMut;
use palette::Srgba;

use crate::ppu::region::Region;

pub mod software;
#[cfg(feature = "vulkan")]
pub mod vulkan;

pub(crate) trait PpuDisplayBackend<R: Region>:
    Send + Sync + Debug + Sized + 'static
{
    type GraphicsApi: GraphicsApi;

    fn new(initialization_data: <Self::GraphicsApi as GraphicsApi>::InitializationData) -> Self;
    fn modify_staging_buffer(&mut self, callback: impl FnOnce(DMatrixViewMut<'_, Srgba<u8>>));
    fn commit_staging_buffer(&mut self);
    fn access_framebuffer(&mut self) -> &<Self::GraphicsApi as GraphicsApi>::FramebufferTexture;
}

pub(crate) trait SupportedGraphicsApiPpu: GraphicsApi {
    type Backend<R: Region>: PpuDisplayBackend<R, GraphicsApi = Self>;
}
