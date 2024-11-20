struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
}
;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
}
;

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.color = model.color;
    out.clip_position = vec4<f32>(model.position, 1.0);
    return out;
}

// Fragment shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let srgb_color = vec4<f32>(in.color, 1.0);
    let rgb_color = pow((srgb_color / vec4<f32>(255.0) + vec4<f32>(0.055)) / vec4<f32>(1.055), vec4<f32>(2.4));
    return vec4<f32>(rgb_color.rgb, 1.0);
}
