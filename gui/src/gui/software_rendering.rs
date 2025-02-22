use egui::FullOutput;
use egui::TextureId;
use nalgebra::{DMatrix, DMatrixViewMut, Matrix2x3, Point2, Vector2, Vector3, Vector4, stack};
use palette::{LinSrgba, Srgba, blend::Compose};
use rayon::iter::IndexedParallelIterator;
use rayon::iter::ParallelIterator;
use std::collections::HashMap;

// FIXME: This is unbearably slow, spending a ton of time in `perp`

#[derive(Copy, Clone, Debug)]
struct Vertex {
    pos: Point2<f32>,
    uv: Point2<f32>,
    color: Srgba<u8>,
}

impl From<egui::epaint::Vertex> for Vertex {
    fn from(vertex: egui::epaint::Vertex) -> Self {
        Vertex {
            pos: Point2::new(vertex.pos.x, vertex.pos.y),
            uv: Point2::new(vertex.uv.x, vertex.uv.y),
            color: Srgba::from_components(vertex.color.to_tuple()),
        }
    }
}

#[derive(Debug, Default)]
pub struct SoftwareEguiRenderer {
    textures: HashMap<TextureId, DMatrix<LinSrgba<f32>>>,
}

impl SoftwareEguiRenderer {
    #[allow(clippy::toplevel_ref_arg)]
    pub fn render(
        &mut self,
        context: &egui::Context,
        mut render_buffer: DMatrixViewMut<Srgba<u8>>,
        full_output: FullOutput,
    ) {
        for remove_texture_id in full_output.textures_delta.free {
            tracing::trace!("Freeing egui texture {:?}", remove_texture_id);
            self.textures.remove(&remove_texture_id);
        }

        for (new_texture_id, new_texture) in full_output.textures_delta.set {
            tracing::debug!("Adding new egui texture {:?}", new_texture_id);

            if new_texture.pos.is_some() && !self.textures.contains_key(&new_texture_id) {
                panic!("Texture not found: {:?}", new_texture_id);
            }

            let texture = self.textures.entry(new_texture_id).or_insert_with(|| {
                let image_size = new_texture.image.size();
                DMatrix::from_element(
                    image_size[0],
                    image_size[1],
                    LinSrgba::new(0.0, 0.0, 0.0, 255.0),
                )
            });

            let source_texture_view = match &new_texture.image {
                egui::ImageData::Color(image) => {
                    let converted_image = image
                        .pixels
                        .clone()
                        .into_iter()
                        .map(|pixel| Srgba::from_components(pixel.to_tuple()).into_linear())
                        .collect();

                    DMatrix::from_vec(image.size[0], image.size[1], converted_image)
                }
                egui::ImageData::Font(font_image) => {
                    let converted_image = font_image
                        .pixels
                        .clone()
                        .into_iter()
                        .map(|coverage| LinSrgba::new(coverage, coverage, coverage, coverage))
                        .collect();

                    DMatrix::from_vec(font_image.size[0], font_image.size[1], converted_image)
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

        render_buffer.fill(Srgba::new(0, 0, 0, 0xff));

        let render_buffer_dimensions =
            Vector2::new(render_buffer.nrows(), render_buffer.ncols()).cast::<f32>();

        for shape in context.tessellate(full_output.shapes, full_output.pixels_per_point) {
            match shape.primitive {
                egui::epaint::Primitive::Mesh(mesh) => {
                    let texture = self.textures.get(&mesh.texture_id).unwrap();

                    for vertex_indexes in mesh.indices.chunks(3) {
                        let [v0, v1, v2]: [Vertex; 3] = [
                            mesh.vertices[vertex_indexes[0] as usize].into(),
                            mesh.vertices[vertex_indexes[1] as usize].into(),
                            mesh.vertices[vertex_indexes[2] as usize].into(),
                        ];

                        let max = Vector2::new(
                            Vector3::new(v0.pos.x, v1.pos.x, v2.pos.x)
                                .max()
                                .min(render_buffer_dimensions.x - 1.0)
                                .round() as usize,
                            Vector3::new(v0.pos.y, v1.pos.y, v2.pos.y)
                                .max()
                                .min(render_buffer_dimensions.y - 1.0)
                                .round() as usize,
                        );

                        let min = Vector2::new(
                            Vector4::new(v0.pos.x, v1.pos.x, v2.pos.x, max.x as f32)
                                .min()
                                .max(0.0)
                                .round() as usize,
                            Vector4::new(v0.pos.y, v1.pos.y, v2.pos.y, max.y as f32)
                                .min()
                                .max(0.0)
                                .round() as usize,
                        );

                        let points = stack![v0.pos.coords, v1.pos.coords, v2.pos.coords];

                        // Precompute edges for the triangle
                        let edges = stack![
                            v1.pos.coords - v0.pos.coords,
                            v2.pos.coords - v1.pos.coords,
                            v0.pos.coords - v2.pos.coords
                        ];

                        let mut bounding_box =
                            render_buffer.view_range_mut(min.x..=max.x, min.y..=max.y);

                        bounding_box
                            .par_column_iter_mut()
                            .enumerate()
                            .map(|(y, row)| (y + min.y, row))
                            .for_each(|(y, mut row)| {
                                for x in min.x..=max.x {
                                    let pixel_center = Point2::new(x as f32 + 0.5, y as f32 + 0.5);

                                    if is_point_in_triangle(pixel_center, points, &edges) {
                                        // Interpolate colors based on barycentric coordinates
                                        let barycentric =
                                            barycentric_coordinates(pixel_center, points, &edges);

                                        let interpolated_color = v0.color.into_linear()
                                            * barycentric.x
                                            + v1.color.into_linear() * barycentric.y
                                            + v2.color.into_linear() * barycentric.z;

                                        let interpolated_uv = v0.uv.coords * barycentric.x
                                            + v1.uv.coords * barycentric.y
                                            + v2.uv.coords * barycentric.z;

                                        let pixel_coords = Point2::new(
                                            (texture.nrows() as f32 * interpolated_uv.x) as usize,
                                            (texture.ncols() as f32 * interpolated_uv.y) as usize,
                                        );

                                        // Inaccuraries that lead outside the texture we will read off with black
                                        let pixel = texture
                                            .get((pixel_coords.x, pixel_coords.y))
                                            .copied()
                                            .unwrap_or_else(|| LinSrgba::new(0.0, 0.0, 0.0, 1.0));

                                        row[x - min.x] = Srgba::from_linear(
                                            (interpolated_color * pixel)
                                                .over(row[x - min.x].into_linear()),
                                        );
                                    }
                                }
                            });
                    }
                }
                egui::epaint::Primitive::Callback(_) => {
                    tracing::warn!("Epaint callbacks are ignored");
                }
            }
        }
    }
}

#[inline]
fn triangle_area(v: Matrix2x3<f32>) -> f32 {
    let edges = Matrix2x3::from_columns(&[
        v.column(1) - v.column(0),
        v.column(2) - v.column(1),
        v.column(0) - v.column(2),
    ]);

    edges.column(0).perp(&(v.column(2) - v.column(0))).abs()
}

#[allow(clippy::toplevel_ref_arg)]
fn barycentric_coordinates(
    point: Point2<f32>,
    v: Matrix2x3<f32>,
    edges: &Matrix2x3<f32>,
) -> Vector3<f32> {
    let area = edges.column(0).perp(&(v.column(2) - v.column(0))).abs();
    let area1 = triangle_area(stack![point.coords, v.column(1), v.column(2)]);
    let area2 = triangle_area(stack![v.column(0), point.coords, v.column(2)]);
    let area3 = triangle_area(stack![v.column(0), v.column(1), point.coords]);

    Vector3::new(area1, area2, area3) / area
}

#[allow(clippy::toplevel_ref_arg)]
fn is_point_in_triangle(point: Point2<f32>, v: Matrix2x3<f32>, edges: &Matrix2x3<f32>) -> bool {
    let to_p = stack![point.coords, point.coords, point.coords] - v;

    let b = Vector3::new(
        edges.column(0).perp(&to_p.column(0)),
        edges.column(1).perp(&to_p.column(1)),
        edges.column(2).perp(&to_p.column(2)),
    );

    b.into_iter().all(|&val| val >= 0.0) || b.into_iter().all(|&val| val <= 0.0)
}
