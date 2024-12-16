struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,

}
;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let vertices = array<vec2<f32>, 6>(vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0), vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, 1.0), vec2<f32>(-1.0, 1.0));

    var out: VertexOutput;
    let pos = vertices[vertex_index];
    out.clip_position = vec4<f32>(pos, 0.0, 1.0);
    return out;
}

// Fragment shader
struct Uniforms {
    resolution: vec2<f32>,
    srgb_surface: f32,
}
;

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;
@group(0) @binding(2)
var<uniform> uniforms: Uniforms;

// References
// https://github.com/gfx-rs/wgpu/issues/2326#issuecomment-1002301171

fn linear_to_srgb(linear: vec3<f32>) -> vec3<f32> {
    let a = 0.055;
    return mix(linear * 12.92, pow(linear, vec3<f32>(1.0 / 2.4)) * (1.0 + a) - vec3<f32>(a), step(vec3<f32>(0.0031308), linear));
}
fn srgb_to_linear(srgb: vec3<f32>) -> vec3<f32> {
    let a = 0.055;
    return mix(srgb / 12.92, pow((srgb + vec3<f32>(a)) / (1.0 + a), vec3<f32>(2.4)), step(vec3<f32>(0.04045), srgb));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_coord = in.clip_position.xy / uniforms.resolution;
    let color = textureSample(t_diffuse, s_diffuse, tex_coord);

    return vec4<f32>(srgb_to_linear(color.rgb), color.a);
}
