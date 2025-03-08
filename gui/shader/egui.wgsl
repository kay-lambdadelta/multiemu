struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) srgba: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) gamma: vec4<f32>,
    @location(1) uv: vec2<f32>,
};

@group(0) @binding(0) var<uniform> screen_size: vec2<f32>;
@group(1) @binding(0) var texture_sampler: sampler;
@group(1) @binding(1) var texture: texture_2d_array<f32>;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;

    output.position = vec4<f32>(
        2.0 * input.position.x / screen_size.x - 1.0,
        1.0 - 2.0 * input.position.y / screen_size.y,
        0.0,
        1.0
    );

    output.gamma = input.srgba / 255.0;
    output.uv = input.uv;

    return output;
}


@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}