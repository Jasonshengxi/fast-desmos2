struct MainUniform {
    location: f32,
}

@group(0) @binding(0)
var<uniform> main_uniform: MainUniform;

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
) -> @builtin(position) vec4<f32> {
    let x = select(-1.0, main_uniform.location, (in_vertex_index & 1u) > 0);
    let y = select(-1.0, 1.0, (in_vertex_index & 2u) > 0);

    return vec4(x, y, 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4(1.0, 0.0, 0.0, 0.0);
}