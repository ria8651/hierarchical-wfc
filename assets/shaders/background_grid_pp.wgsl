#import bevy_core_pipeline::fullscreen_vertex_shader FullscreenVertexOutput



struct CustomMaterial {
    color: vec4<f32>,
};
@group(1) @binding(0)
var<uniform> material: CustomMaterial;


@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    return  vec4<f32>(0.5, 0.5, 1.0, 0.5);
}

