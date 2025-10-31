use egui::{FullOutput, TextureId};
use nalgebra::{DMatrix, DMatrixViewMut, Point2, Scalar, Vector2, Vector3, Vector4};
use palette::{
    Srgba,
    cast::{ComponentOrder, Packed},
    named::BLACK,
};
use std::collections::HashMap;

mod render_pixel;

// NOTE: https://github.com/emilk/egui/pull/2071

#[derive(Copy, Clone, Debug, PartialEq)]
struct Vertex {
    position: Point2<f32>,
    uv: Point2<f32>,
    color: Srgba<f32>,
}

impl From<egui::epaint::Vertex> for Vertex {
    fn from(vertex: egui::epaint::Vertex) -> Self {
        Vertex {
            position: Point2::new(vertex.pos.x, vertex.pos.y),
            uv: Point2::new(vertex.uv.x, vertex.uv.y),
            color: Srgba::from_components(vertex.color.to_tuple()).into_format(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
struct Triangle {
    v0: Vertex,
    v1: Vertex,
    v2: Vertex,
    edge0: Vector2<f32>,
    edge1: Vector2<f32>,
    edge2: Vector2<f32>,
    area: f32,
}

impl Triangle {
    fn new(v0: Vertex, v1: Vertex, v2: Vertex) -> Self {
        let edge0 = v0.position - v1.position;
        let area = edge0.perp(&(v2.position - v0.position)).abs() / 2.0;

        Triangle {
            v0,
            v1,
            v2,
            edge0,
            edge1: v1.position - v2.position,
            edge2: v2.position - v0.position,
            area,
        }
    }
}

#[derive(Debug, Default)]
/// A somewhat fast egui software renderer
pub struct SoftwareEguiRenderer {
    textures: HashMap<TextureId, DMatrix<Srgba<f32>>>,
}

impl SoftwareEguiRenderer {
    #[allow(clippy::toplevel_ref_arg)]
    /// Render to a surface given the pixel order
    pub fn render<P: ComponentOrder<Srgba<u8>, u32> + Scalar + Send + Sync>(
        &mut self,
        context: &egui::Context,
        mut render_buffer: DMatrixViewMut<Packed<P, u32>>,
        full_output: FullOutput,
    ) {
        for remove_texture_id in full_output.textures_delta.free {
            tracing::trace!("Freeing egui texture {:?}", remove_texture_id);
            self.textures.remove(&remove_texture_id);
        }

        for (new_texture_id, new_texture) in full_output.textures_delta.set {
            tracing::debug!("Adding new egui texture {:?}", new_texture_id);

            assert!(
                !(new_texture.pos.is_some() && !self.textures.contains_key(&new_texture_id)),
                "Texture not found: {new_texture_id:?}"
            );

            let texture = self.textures.entry(new_texture_id).or_insert_with(|| {
                let image_size = new_texture.image.size();
                DMatrix::from_element(image_size[0], image_size[1], BLACK.into_format().into())
            });

            let source_texture_view = match &new_texture.image {
                egui::ImageData::Color(image) => {
                    let converted_image = image
                        .pixels
                        .clone()
                        .into_iter()
                        .map(|pixel| Srgba::from_components(pixel.to_tuple()).into_format())
                        .collect();

                    DMatrix::from_vec(image.size[0], image.size[1], converted_image)
                }
            };

            let texture_update_offset = Vector2::from(new_texture.pos.unwrap_or([0, 0]));

            let mut destination_texture_view = texture.view_range_mut(
                texture_update_offset.x
                    ..(texture_update_offset.x + source_texture_view.nrows()).min(texture.nrows()),
                texture_update_offset.y
                    ..(texture_update_offset.y + source_texture_view.ncols()).min(texture.ncols()),
            );

            destination_texture_view.copy_from(&source_texture_view);
        }

        render_buffer.fill(Packed::pack(BLACK.into()));

        let render_buffer_dimensions =
            Vector2::new(render_buffer.nrows(), render_buffer.ncols()).cast::<f32>();

        for shape in context.tessellate(full_output.shapes, full_output.pixels_per_point) {
            match shape.primitive {
                egui::epaint::Primitive::Mesh(mesh) => {
                    let texture = self.textures.get(&mesh.texture_id).unwrap();

                    let texture_dimensions =
                        Vector2::new(texture.nrows() as f32, texture.ncols() as f32);

                    for vertex_indexes in mesh.indices.chunks_exact(3) {
                        let [v0, v1, v2]: [Vertex; 3] = [
                            mesh.vertices[vertex_indexes[0] as usize].into(),
                            mesh.vertices[vertex_indexes[1] as usize].into(),
                            mesh.vertices[vertex_indexes[2] as usize].into(),
                        ];

                        let max = Vector2::new(
                            Vector3::new(v0.position.x, v1.position.x, v2.position.x)
                                .max()
                                .min(render_buffer_dimensions.x - 1.0)
                                as usize,
                            Vector3::new(v0.position.y, v1.position.y, v2.position.y)
                                .max()
                                .min(render_buffer_dimensions.y - 1.0)
                                as usize,
                        );

                        let min = Vector2::new(
                            Vector4::new(v0.position.x, v1.position.x, v2.position.x, max.x as f32)
                                .min()
                                .max(0.0) as usize,
                            Vector4::new(v0.position.y, v1.position.y, v2.position.y, max.y as f32)
                                .min()
                                .max(0.0) as usize,
                        );

                        let triangle = Triangle::new(v0, v1, v2);

                        let mut bounding_box =
                            render_buffer.view_range_mut(min.x..=max.x, min.y..=max.y);

                        for x in min.x..=max.x {
                            for y in min.y..=max.y {
                                let destination_pixel =
                                    bounding_box.get_mut((x - min.x, y - min.y)).unwrap();

                                render_pixel::render_pixel(
                                    Point2::new(x as f32, y as f32),
                                    &triangle,
                                    texture,
                                    texture_dimensions,
                                    destination_pixel,
                                );
                            }
                        }
                    }
                }
                egui::epaint::Primitive::Callback(_) => {
                    tracing::warn!("Epaint callbacks are ignored");
                }
            }
        }
    }
}
