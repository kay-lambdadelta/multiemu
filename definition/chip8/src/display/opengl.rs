use crate::display::{draw_sprite_common, Chip8Display, Chip8DisplayBackend};
use crossbeam::channel::Sender;
use glium::buffer::{Buffer, BufferMode, BufferType};
use glium::texture::{ClientFormat, RawImage2d};
use glium::Texture2d;
use multiemu_machine::display::opengl::OpenGlRendering;
use multiemu_machine::display::RenderBackend;
use nalgebra::{DMatrix, DMatrixViewMut, Point2};
use palette::Srgba;
use std::borrow::Cow;
use std::cell::RefCell;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use std::sync::Arc;

struct OpenGlState {
    pub staging_buffer: RefCell<Buffer<[Srgba<u8>]>>,
    pub render_image: RefCell<Rc<Texture2d>>,
    pub frame_sender: Sender<Rc<Texture2d>>,
}

impl Chip8DisplayBackend for OpenGlState {
    fn draw_sprite(&self, position: Point2<u8>, sprite: &[u8]) -> bool {
        let mut staging_buffer = self.staging_buffer.borrow_mut();
        let mut staging_buffer = staging_buffer.map();

        let staging_buffer = DMatrixViewMut::from_slice(staging_buffer.deref_mut(), 64, 32);

        draw_sprite_common(position, sprite, staging_buffer)
    }

    fn clear_display(&self) {
        let mut staging_buffer = self.staging_buffer.borrow_mut();
        let mut staging_buffer = staging_buffer.map();

        staging_buffer.fill(Srgba::new(0, 0, 0, 0xff));
    }

    fn save_screen_contents(&self) -> DMatrix<Srgba<u8>> {
        let mut staging_buffer = self.staging_buffer.borrow_mut();
        let staging_buffer = staging_buffer.map();

        DMatrix::from_row_slice(64, 32, staging_buffer.deref())
    }

    fn load_screen_contents(&self, buffer: DMatrix<Srgba<u8>>) {
        let mut staging_buffer = self.staging_buffer.borrow_mut();
        let mut staging_buffer = staging_buffer.map();

        staging_buffer.copy_from_slice(buffer.as_slice());
    }

    fn commit_display(&self) {
        // Borrow the staging buffer
        let mut staging_buffer = self.staging_buffer.borrow_mut();
        let staging_buffer = staging_buffer.map();

        // Create a glium texture from the staging buffer
        let image = RawImage2d {
            data: Cow::Borrowed(bytemuck::cast_slice::<_, u8>(staging_buffer.deref())),
            width: 64,
            height: 32,
            format: ClientFormat::U8U8U8U8,
        };

        let render_image = self.render_image.borrow();

        // Update the render image with the new data
        render_image.write(
            glium::Rect {
                left: 0,
                bottom: 0,
                width: 64,
                height: 32,
            },
            image,
        );

        self.frame_sender.try_send(render_image.clone()).unwrap();
    }
}

pub fn set_display_data(
    display: &Chip8Display,
    initialization_data: Arc<<OpenGlRendering as RenderBackend>::ComponentInitializationData>,
    frame_sender: Sender<<OpenGlRendering as RenderBackend>::ComponentFramebuffer>,
) {
    let initial_contents = DMatrix::from_element(64, 32, Srgba::new(0, 0, 0, 255));
    let staging_buffer = RefCell::new(
        Buffer::new(
            &initialization_data.context,
            initial_contents.as_slice(),
            BufferType::ArrayBuffer,
            BufferMode::Dynamic,
        )
        .unwrap(),
    );

    let render_image = Rc::new(
        Texture2d::new(
            &initialization_data.context,
            RawImage2d {
                data: Cow::Borrowed(bytemuck::cast_slice::<_, u8>(initial_contents.as_slice())),
                width: 64,
                height: 32,
                format: ClientFormat::U8U8U8U8,
            },
        )
        .unwrap(),
    );

    let _ = display.state.set(Box::new(OpenGlState {
        staging_buffer,
        render_image: RefCell::new(render_image.clone()),
        frame_sender,
    }));
}
