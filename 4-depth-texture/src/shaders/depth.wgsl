struct VertexInput {
    @location(0) position: vec3<f32>,
}
;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
}
;

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(model.position, 1.0);
    return out;
}

// Fragment shader
struct Uniforms {
    @align(8) resolution: vec2<f32>,
}
;

@group(0) @binding(0)
var t_depth: texture_depth_2d;
@group(0) @binding(1)
var s_depth: sampler;
@group(0) @binding(2)
var<uniform> uniforms: Uniforms;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_coord = in.clip_position.xy / uniforms.resolution;
    let depth = textureSample(t_depth, s_depth, tex_coord);
    return vec4<f32>(depth, depth, depth, 1.0);
}
