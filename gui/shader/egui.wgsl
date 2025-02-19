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

struct ScreenSize {
    screen_size: vec2<f32>,
};

@group(0) @binding(0) var<uniform> screen_size: ScreenSize;
@group(1) @binding(0) var texture_sampler: sampler;
@group(1) @binding(1) var texture: texture_2d_array<f32>;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;

    output.position = vec4<f32>(
        2.0 * input.position.x / screen_size.screen_size.x - 1.0,
        1.0 - 2.0 * input.position.y / screen_size.screen_size.y,
        0.0,
        1.0
    );

    output.gamma = input.srgba / 255.0;
    output.uv = input.uv;

    return output;
}

fn srgb_from_linear(rgb: vec3<f32>) -> vec3<f32> {
    let cutoff = rgb < vec3(0.0031308);
    let lower = rgb * vec3(3294.6);
    let higher = vec3(269.025) * pow(rgb, vec3(1.0 / 2.4)) - vec3(14.025);
    return select(higher, lower, cutoff);
}

fn srgba_from_linear(rgba: vec4<f32>) -> vec4<f32> {
    return vec4(srgb_from_linear(rgba.rgb), 255.0 * rgba.a);
}

fn gamma_from_linear_rgba(linear_rgba: vec4<f32>) -> vec4<f32> {
    return vec4(srgb_from_linear(linear_rgba.rgb) / 255.0, linear_rgba.a);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let texture_in_gamma = gamma_from_linear_rgba(textureSample(texture, texture_sampler, input.uv, 0));
    return input.gamma * texture_in_gamma;
}