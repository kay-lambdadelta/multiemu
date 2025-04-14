struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    // sRGB
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@group(0) @binding(0) var<uniform> screen_size: vec2<f32>;
@group(0) @binding(1) var texture_sampler: sampler;
@group(1) @binding(0) var texture: texture_2d<f32>;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;

    output.position = vec4<f32>(
        2.0 * input.position.x / screen_size.x - 1.0,
        1.0 - 2.0 * input.position.y / screen_size.y,
        0.0,
        1.0
    );

    output.color = input.color;
    output.uv = input.uv;

    return output;
}


// 0-1 linear  from  0-1 sRGB gamma
fn linear_from_gamma_rgb(srgb: vec3<f32>) -> vec3<f32> {
    let cutoff = srgb < vec3<f32>(0.04045);
    let lower = srgb / vec3<f32>(12.92);
    let higher = pow((srgb + vec3<f32>(0.055)) / vec3<f32>(1.055), vec3<f32>(2.4));
    return select(higher, lower, cutoff);
}

// 0-1 sRGB gamma  from  0-1 linear
fn gamma_from_linear_rgb(rgb: vec3<f32>) -> vec3<f32> {
    let cutoff = rgb < vec3<f32>(0.0031308);
    let lower = rgb * vec3<f32>(12.92);
    let higher = vec3<f32>(1.055) * pow(rgb, vec3<f32>(1.0 / 2.4)) - vec3<f32>(0.055);
    return select(higher, lower, cutoff);
}

// 0-1 sRGBA gamma  from  0-1 linear
fn gamma_from_linear_rgba(linear_rgba: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(gamma_from_linear_rgb(linear_rgba.rgb), linear_rgba.a);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let texture_color = textureSample(texture, texture_sampler, input.uv);
    let srgba_texture_color = gamma_from_linear_rgba(texture_color);
    let srgba_vertex_color = gamma_from_linear_rgba(input.color);
    let blended_color = srgba_texture_color * srgba_vertex_color;
    
    return vec4<f32>(linear_from_gamma_rgb(blended_color.rgb), blended_color.a);
}