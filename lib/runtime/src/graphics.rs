use multiemu_graphics::GraphicsApi;

#[allow(clippy::type_complexity)]
pub trait GraphicsCallback<R: GraphicsApi>: 'static {
    fn get_framebuffer<'a>(&'a self, callback: Box<dyn FnOnce(&R::ComponentFramebuffer) + 'a>);
}
