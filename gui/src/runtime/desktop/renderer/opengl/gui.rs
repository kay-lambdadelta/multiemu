use encase::{internal::WriteInto, ShaderSize};
use glium::{
    backend::Context,
    uniforms::{EmptyUniforms, UniformBuffer, UniformsStorage},
    BackfaceCullingMode, Blend, Display, DrawParameters, Frame, Program, Surface,
};
use glutin::surface::WindowSurface;
use nalgebra::{Point2, Vector2};
use palette::{
    named::{BLACK, BLUE, RED},
    Srgba, WithAlpha,
};
use std::{cell::LazyCell, rc::Rc, sync::LazyLock};

include!(concat!(env!("OUT_DIR"), "/egui.rs"));

pub struct OpenglEguiRenderer {
    context: Rc<Context>,
    program: Program,
    draw_parameters: DrawParameters<'static>,
}

impl OpenglEguiRenderer {
    pub fn new(context: Rc<Context>) -> Self {
        let program = Program::from_source(
            &context,
            shader::glsl::VERTEX_SHADER,
            shader::glsl::FRAGMENT_SHADER,
            None,
        )
        .unwrap();

        Self {
            context,
            program,
            draw_parameters: DrawParameters {
                blend: Blend::alpha_blending(),
                backface_culling: BackfaceCullingMode::CullingDisabled,
                ..Default::default()
            },
        }
    }

    pub fn render(
        &mut self,
        context: &egui::Context,
        render_buffer: &mut Frame,
        full_output: egui::FullOutput,
    ) {
        for remove_texture_id in full_output.textures_delta.free {
            tracing::trace!("Freeing egui texture {:?}", remove_texture_id);
        }

        let screen_size = shader::types::ScreenSize {
            screen_size: Vector2::new(
                render_buffer.get_dimensions().0,
                render_buffer.get_dimensions().1,
            )
            .cast(),
        };

        render_buffer.clear(
            None,
            Some(BLACK.into_linear().with_alpha(1.0).into_components()),
            true,
            None,
            None,
        );

        let mut screen_size_buffer =
            encase::UniformBuffer::new([0; shader::types::ScreenSize::SHADER_SIZE.get() as usize]);
        screen_size_buffer.write(&screen_size).unwrap();

    }
}
