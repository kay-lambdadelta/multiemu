use encase::ShaderSize;
use glium::{
    BackfaceCullingMode, Blend, DrawParameters, Frame, Program, Surface, backend::Context,
};
use nalgebra::Vector2;
use palette::{WithAlpha, named::BLACK};
use std::rc::Rc;

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
    }
}
