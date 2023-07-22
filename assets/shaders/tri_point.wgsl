#import bevy_pbr::mesh_vertex_output MeshVertexOutput

struct CustomMaterial {
    color: vec4<f32>,
};

@group(1) @binding(0)
var<uniform> material: CustomMaterial;

@fragment
fn fragment(
    mesh: MeshVertexOutput,
) -> @location(0) vec4<f32> {

    let uv_dx = dpdx(mesh.uv);
    let uv_dy = dpdy(mesh.uv);

    let signed_distance: f32 = (0.5 - sqrt(dot(mesh.uv, mesh.uv))) / uv_dx.x;
    return mesh.color * vec4<f32>(1.0, 1.0, 1.0, clamp(signed_distance, 0.0, 1.0));
}
