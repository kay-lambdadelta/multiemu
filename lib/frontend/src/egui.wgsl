struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
};

@group(0) @binding(0) var<uniform> screen_size: vec2<f32>;
@group(0) @binding(1) var texture_sampler: sampler;
@group(1) @binding(0) var texture: texture_2d<f32>;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    let ndc_x = 2.0 * input.position.x / screen_size.x - 1.0;
    let ndc_y = 1.0 - 2.0 * input.position.y / screen_size.y;

    return VertexOutput(
        vec4<f32>(ndc_x, ndc_y, 0.0, 1.0),
        input.color,
        input.uv,
    );
}

fn srgb_to_linear(srgb: vec3<f32>) -> vec3<f32> {
    let cutoff = srgb < vec3<f32>(0.04045);
    let lower = srgb / 12.92;
    let higher = pow((srgb + 0.055) / 1.055, vec3<f32>(2.4));
    return select(higher, lower, cutoff);
}

fn linear_to_srgb(rgb: vec3<f32>) -> vec3<f32> {
    let cutoff = rgb < vec3<f32>(0.0031308);
    let lower = rgb * 12.92;
    let higher = 1.055 * pow(rgb, vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(higher, lower, cutoff);
}

fn linear_to_srgba(rgba: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(linear_to_srgb(rgba.rgb), rgba.a);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color_linear = textureSample(texture, texture_sampler, input.uv);
    let tex_color_srgb = linear_to_srgba(tex_color_linear);

    let blended_srgb = tex_color_srgb * input.color;

    return vec4<f32>(
        srgb_to_linear(blended_srgb.rgb),
        blended_srgb.a
    );
}
