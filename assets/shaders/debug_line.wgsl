#import bevy_pbr::mesh_bindings   mesh
#import bevy_pbr::mesh_functions  mesh_position_local_to_clip
#import bevy_pbr::mesh_view_bindings view
#import bevy_pbr::mesh_view_bindings globals

struct DebugLineMaterial {
    color: vec4<f32>,
};
@group(1) @binding(0)
var<uniform> material: DebugLineMaterial;

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(4) color: vec4<f32>,

};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec3<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    // var view_view = view.projection * view.inverse_projection * view.view_proj ;
    // var view_v = mat3x3<f32>(
    //     view_view[0].xyz,
    //     view_view[1].xyz,
    //     view_view[2].xyz
    // );


    // var local_depth = normalize(
    //     vec3<f32>(0.0, 1.0, 0.0) * mat3x3<f32>(
    //         view_view[0].xyz,
    //         view_view[1].xyz,
    //         view_view[2].xyz
    //     )
    // );
    var origin = view.world_position;
    let tangent: vec3<f32> = -0.1 * cross(vertex.normal, normalize(vertex.position - origin.xyz));


    let ss_tangent = mesh_position_local_to_clip(
        mesh.model,
        vec4<f32>(tangent, 0.0),
    ); //+ 0.05 * tangent_a;


    out.clip_position = mesh_position_local_to_clip(
        mesh.model,
        vec4<f32>(vertex.position, 1.0),
    ) + ss_tangent; //+ 0.05 * tangent_a;
    out.uv = vertex.uv;
    out.color = vertex.color.xyz;
    return out;
}

struct FragmentInput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec3<f32>,
};

@fragment
fn fragment(input: FragmentInput) -> @location(0) vec4<f32> {
    let dash_frequency = 5.0;
    let dash_velocity = 0.25;

    let color = input.color;
    let mix_dashes = select(0.0, 1.0, fract(dash_velocity * dash_frequency * globals.time - dash_frequency * input.uv.x) > 0.5);
    let mix_radial = select(0.0, 1.0, 0.5 - abs(input.uv.y - 0.5) > 0.25);
    return  vec4<f32>(color, mix_dashes * mix_radial) ; //material.color;
}