use crate::tia::region::Region;
use multiemu_graphics::GraphicsApi;
use nalgebra::DMatrixViewMut;
use palette::Srgba;
use std::fmt::Debug;

pub mod software;
#[cfg(feature = "vulkan")]
pub mod vulkan;

pub(crate) trait TiaDisplayBackend<R: Region>: Debug + Sized + 'static {
    type GraphicsApi: GraphicsApi;

    fn new(initialization_data: <Self::GraphicsApi as GraphicsApi>::InitializationData) -> Self;
    fn modify_staging_buffer(&mut self, callback: impl FnOnce(DMatrixViewMut<'_, Srgba<u8>>));
    fn commit_staging_buffer(&mut self);
    fn access_framebuffer(
        &mut self,
        callback: impl FnOnce(&<Self::GraphicsApi as GraphicsApi>::FramebufferTexture),
    );
}

pub(crate) trait SupportedGraphicsApiTia: GraphicsApi {
    type Backend<R: Region>: TiaDisplayBackend<R, GraphicsApi = Self>;
}
