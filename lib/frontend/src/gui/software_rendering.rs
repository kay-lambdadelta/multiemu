use std::collections::HashMap;

use egui::{FullOutput, TextureId};
use nalgebra::{DMatrix, DMatrixView, DMatrixViewMut, Point2, Scalar, Vector2, Vector3, Vector4};
use palette::{
    Srgba, WithAlpha,
    blend::Compose,
    cast::{ComponentOrder, Packed},
    named::BLACK,
};
use rayon::{
    iter::{
        IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator, ParallelIterator,
    },
    slice::ParallelSlice,
};

// NOTE: https://github.com/emilk/egui/pull/2071
//
// ^^ Read that before touching this

const TILE_SIZE: usize = 32;

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
struct Triangle<'a> {
    // Vertexes
    v0: Vertex,
    v1: Vertex,
    v2: Vertex,

    // Edges
    edge0: Vector2<f32>,
    edge1: Vector2<f32>,
    edge2: Vector2<f32>,

    // Bounding box
    min: Point2<usize>,
    max: Point2<usize>,

    // Texture this triangle should be skinned with
    texture: &'a DMatrix<Srgba<f32>>,
}

impl<'a> Triangle<'a> {
    fn new(
        v0: Vertex,
        v1: Vertex,
        v2: Vertex,
        render_buffer_dimensions: Vector2<f32>,
        texture: &'a DMatrix<Srgba<f32>>,
    ) -> Self {
        let edge0 = v0.position - v1.position;

        let max = Vector2::new(
            Vector3::new(v0.position.x, v1.position.x, v2.position.x)
                .max()
                .min(render_buffer_dimensions.x - 1.0) as usize,
            Vector3::new(v0.position.y, v1.position.y, v2.position.y)
                .max()
                .min(render_buffer_dimensions.y - 1.0) as usize,
        );

        let min = Vector2::new(
            Vector4::new(v0.position.x, v1.position.x, v2.position.x, max.x as f32)
                .min()
                .max(0.0) as usize,
            Vector4::new(v0.position.y, v1.position.y, v2.position.y, max.y as f32)
                .min()
                .max(0.0) as usize,
        );

        Triangle {
            v0,
            v1,
            v2,
            edge0,
            edge1: v1.position - v2.position,
            edge2: v2.position - v0.position,
            min: min.into(),
            max: max.into(),
            texture,
        }
    }
}

#[derive(Debug, Default)]
/// A somewhat fast egui software renderer
pub struct SoftwareEguiRenderer {
    textures: HashMap<TextureId, DMatrix<Srgba<f32>>>,
}

impl SoftwareEguiRenderer {
    /// Render to a surface given the pixel order
    pub fn render<P: ComponentOrder<Srgba<u8>, u32> + Scalar + Send + Sync>(
        &mut self,
        context: &egui::Context,
        mut render_buffer: DMatrixViewMut<Packed<P, u32>>,
        full_output: FullOutput,
    ) {
        for (new_texture_id, new_texture) in full_output.textures_delta.set {
            assert!(
                new_texture.is_whole() || self.textures.contains_key(&new_texture_id),
                "Texture not found: {new_texture_id:?}"
            );

            if new_texture.is_whole() {
                self.textures.remove(&new_texture_id);
            }

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

            assert_eq!(
                source_texture_view.shape(),
                destination_texture_view.shape()
            );

            destination_texture_view.copy_from(&source_texture_view);
        }

        let render_buffer_dimensions = Vector2::new(render_buffer.nrows(), render_buffer.ncols());

        let triangles: Vec<_> = context
            .tessellate(full_output.shapes, full_output.pixels_per_point)
            .par_iter()
            .flat_map(|shape| match &shape.primitive {
                egui::epaint::Primitive::Mesh(mesh) => {
                    let texture = self.textures.get(&mesh.texture_id).unwrap();

                    mesh.indices.par_chunks_exact(3).map(|vertex_indexes| {
                        let [mut v0, mut v1, mut v2]: [Vertex; 3] = [
                            mesh.vertices[vertex_indexes[0] as usize].into(),
                            mesh.vertices[vertex_indexes[1] as usize].into(),
                            mesh.vertices[vertex_indexes[2] as usize].into(),
                        ];

                        // Scale for our physical screen dimensions
                        v0.position *= full_output.pixels_per_point;
                        v1.position *= full_output.pixels_per_point;
                        v2.position *= full_output.pixels_per_point;

                        Triangle::new(v0, v1, v2, render_buffer_dimensions.cast::<f32>(), texture)
                    })
                }
                egui::epaint::Primitive::Callback(_) => {
                    unreachable!("Epaint callbacks should not be sent");
                }
            })
            .collect();

        let tiles_dim = render_buffer_dimensions.map(|a| a.div_ceil(TILE_SIZE));

        let grouped_triangles_buffer: Vec<Vec<_>> = (0..tiles_dim.product())
            .into_par_iter()
            .map(|index| {
                let tile_position = Point2::new(index % tiles_dim.x, index / tiles_dim.x);

                let tile_min = tile_position.coords * TILE_SIZE;
                let tile_max = tile_min.add_scalar(TILE_SIZE);

                triangles
                    .par_iter()
                    .enumerate()
                    .filter_map(|(index, triangle)| {
                        let overlaps = !(triangle.max.x < tile_min.x
                            || triangle.min.x > tile_max.x
                            || triangle.max.y < tile_min.y
                            || triangle.min.y > tile_max.y);

                        if overlaps { Some(index) } else { None }
                    })
                    .collect()
            })
            .collect();

        let grouped_triangles =
            DMatrixView::from_slice(&grouped_triangles_buffer, tiles_dim.x, tiles_dim.y);

        render_buffer
            .par_column_iter_mut()
            .enumerate()
            .for_each(|(y, mut column)| {
                for (x, destination_pixel) in column.iter_mut().enumerate() {
                    let point = Point2::new(x, y);
                    let tile_position = point / TILE_SIZE;

                    let in_tile_triangles = &grouped_triangles[(tile_position.x, tile_position.y)];

                    let active_triangles = in_tile_triangles.iter().filter_map(|index| {
                        let triangle = &triangles[*index];

                        if (triangle.min.x..=triangle.max.x).contains(&x)
                            && (triangle.min.y..=triangle.max.y).contains(&y)
                        {
                            Some(triangle)
                        } else {
                            None
                        }
                    });

                    for triangle in active_triangles {
                        let source_pixel = calculate_source_pixel(point.cast(), triangle);

                        // Eliminate useless blending
                        if source_pixel.alpha > 0.0 {
                            *destination_pixel = Packed::pack(Srgba::from_format(
                                source_pixel.over(destination_pixel.unpack().into_format()),
                            ));
                        }
                    }
                }
            });

        for remove_texture_id in full_output.textures_delta.free {
            tracing::trace!("Freeing egui texture {:?}", remove_texture_id);
            self.textures.remove(&remove_texture_id);
        }
    }
}

#[inline]
fn calculate_source_pixel(position: Point2<f32>, triangle: &Triangle) -> Srgba<f32> {
    let texture_dimensions = Vector2::new(
        triangle.texture.nrows() as f32,
        triangle.texture.ncols() as f32,
    );

    let pixel_center = Point2::new(position.x, position.y) + Vector2::from_element(0.5);

    // Interpolate colors based on barycentric coordinates
    let barycentric = barycentric_coordinates(pixel_center, triangle);

    if is_inside_triangle(barycentric) {
        let interpolated_color = triangle.v0.color * barycentric.x
            + triangle.v1.color * barycentric.y
            + triangle.v2.color * barycentric.z;

        let interpolated_uv = triangle.v0.uv.coords * barycentric.x
            + triangle.v1.uv.coords * barycentric.y
            + triangle.v2.uv.coords * barycentric.z;

        let pixel_coords = Point2::new(
            (texture_dimensions.x * interpolated_uv.x) as usize,
            (texture_dimensions.y * interpolated_uv.y) as usize,
        );

        // Inaccuracies that lead outside the texture we will read off with black
        let pixel = triangle
            .texture
            .get((pixel_coords.x, pixel_coords.y))
            .unwrap_or(&const { Srgba::new(0.0, 0.0, 0.0, 1.0) });

        interpolated_color * *pixel
    } else {
        BLACK.with_alpha(0.0).into_format()
    }
}

#[inline]
fn barycentric_coordinates(point: Point2<f32>, triangle: &Triangle) -> Vector3<f32> {
    let v0p = triangle.v0.position - point;
    let v1p = triangle.v1.position - point;
    let v2p = triangle.v2.position - point;

    let area = Vector3::new(v1p.perp(&v2p), v2p.perp(&v0p), v0p.perp(&v1p));

    let signed_double_area = (-triangle.edge0).perp(&triangle.edge2);

    if signed_double_area.abs() < f32::EPSILON {
        return Vector3::default();
    }

    area / signed_double_area
}

#[inline]
fn is_inside_triangle(coords: Vector3<f32>) -> bool {
    coords.into_iter().all(|&val| val >= 0.0) || coords.into_iter().all(|&val| val <= 0.0)
}
