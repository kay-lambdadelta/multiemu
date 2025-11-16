use nalgebra::{DMatrix, Point2, Scalar, Vector2, Vector3};
use palette::{
    Srgba,
    blend::Compose,
    cast::{ComponentOrder, Packed},
    named::BLACK,
};

use super::Triangle;

pub(super) fn render_pixel<P: ComponentOrder<Srgba<u8>, u32> + Scalar>(
    position: Point2<f32>,
    triangle: &Triangle,
    texture: &DMatrix<Srgba<f32>>,
    texture_dimensions: Vector2<f32>,
    destination_pixel: &mut Packed<P, u32>,
) {
    let pixel_center = Point2::new(position.x, position.y) + Vector2::from_element(0.5);

    if is_point_in_triangle(pixel_center, triangle) {
        // Interpolate colors based on barycentric coordinates
        let barycentric = barycentric_coordinates(pixel_center, triangle);

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
        let pixel = texture
            .get((pixel_coords.x, pixel_coords.y))
            .copied()
            .unwrap_or(BLACK.into_format().into());

        let source_pixel = interpolated_color * pixel;

        *destination_pixel = Packed::pack(Srgba::from_format(
            source_pixel.over(destination_pixel.unpack().into_format()),
        ));
    }
}

fn barycentric_coordinates(point: Point2<f32>, triangle: &Triangle) -> Vector3<f32> {
    let area1 = (triangle.v1.position - point).perp(&(triangle.v2.position - point));
    let area2 = (triangle.v2.position - point).perp(&(triangle.v0.position - point));
    let area3 = (triangle.v0.position - point).perp(&(triangle.v1.position - point));

    Vector3::new(area1, area2, area3).abs() / (2.0 * triangle.area)
}

fn is_point_in_triangle(point: Point2<f32>, triangle: &Triangle) -> bool {
    let to_p0 = point - triangle.v0.position;
    let to_p1 = point - triangle.v1.position;
    let to_p2 = point - triangle.v2.position;

    let b = Vector3::new(
        triangle.edge0.perp(&to_p0),
        triangle.edge1.perp(&to_p1),
        triangle.edge2.perp(&to_p2),
    );

    b.into_iter().all(|&val| val >= 0.0) || b.into_iter().all(|&val| val <= 0.0)
}
