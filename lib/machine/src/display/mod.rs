use backend::RenderBackend;
use std::{fmt::Debug, sync::Mutex};

pub mod backend;
pub mod shader;

/// Display components that need to swap out framebuffers they use this to make the runtime aware they are using a new framebuffer
pub struct FrameReceptacle<R: RenderBackend>(Mutex<Option<R::ComponentFramebuffer>>);

impl<R: RenderBackend> Default for FrameReceptacle<R> {
    fn default() -> Self {
        Self(Mutex::new(None))
    }
}

impl<R: RenderBackend> Debug for FrameReceptacle<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FrameReceptacle").finish()
    }
}

impl<R: RenderBackend> FrameReceptacle<R> {
    pub fn submit(&self, framebuffer: R::ComponentFramebuffer) {
        *self.0.lock().unwrap() = Some(framebuffer);
    }

    pub fn get(&self) -> Option<R::ComponentFramebuffer> {
        self.0.lock().unwrap().take()
    }
}
