struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
}


@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
    @location(0) vertex_pos: vec2<f32>,
) -> VertexOutput {
    var output: VertexOutput;

    output.position = vec4<f32>(vertex_pos, 0.0, 1.0);
    output.tex_coord = (vertex_pos + 1.0) / 2.0;

    return output;
}

@group(0) @binding(0)
var thing_texture: texture_2d<f32>;
@group(0) @binding(1)
var thing_sampler: sampler;

@fragment
fn fs_main(vo: VertexOutput) -> @location(0) vec4<f32> {
    let sampled = textureSample(thing_texture, thing_sampler, vo.tex_coord);
    return vec4<f32>(sampled.rrr, 0.0);
}