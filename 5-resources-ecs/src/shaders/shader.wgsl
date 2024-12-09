struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) tex_coords: vec2<f32>,
}
;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,

}
;

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.color = model.color;
    out.tex_coords = model.tex_coords;
    out.clip_position = vec4<f32>(model.position, 1.0);
    return out;
}

// Fragment shader
@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = in.color;
    let tex_coords = in.tex_coords;
    let tex_color = textureSample(t_diffuse, s_diffuse, tex_coords);

    // Combine color and texture
    let final_color = color * tex_color.rgb;

    // Apply sRGB conversion formula correctly
    let rgb_color = (final_color + 0.055) / 1.055;
    let corrected_rgb_color = pow(rgb_color, vec3<f32>(2.4));
    return vec4<f32>(corrected_rgb_color, 1.0);
}
