

// #import bevy_core_pipeline::fullscreen_vertex_shader FullscreenVertexOutput
#import bevy_sprite::mesh2d_bindings        mesh
#import bevy_sprite::mesh2d_view_bindings  view
struct CustomMaterial {
    color: vec4<f32>,
};
@group(1) @binding(0)
var<uniform> material: CustomMaterial;

struct Vertex {
    @location(0) position: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) coord: vec4<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(vertex.position, 1.0);
    // out.position = mesh2d_position_local_to_clip(
    //     mesh.model,
    //     vec4<f32>(vertex.position.xyz, 1.0)
    // );
    //vec4<f32>(vertex.position.xyz, 1.0);
    // out.coord = vec4<f32>(vertex.position.xyz, 1.);

    // out.coord = mesh.model * out.position;
    let x_basis = view.view_proj * vec4<f32>(1.0, 0.0, 0.0, 0.0);
    let y_basis = view.view_proj * vec4<f32>(0.0, 1.0, 0.0, 0.0);
    let scale = vec4<f32>(1.0 / x_basis.x, 0.0, 0.0, 0.0) + vec4<f32>(0.0, 1.0 / y_basis.y, 0.0, 0.0);
    let coord = scale * vec4(vertex.position.xy, 0.0, 0.0);

    let origin = view.view_proj * vec4<f32>(0.0, 0.0, 0.0, 1.0);
    out.coord = coord - origin * scale;

    return out;
}


struct FragmentInput {
    @location(0) coord: vec4<f32>,
};


fn grid(uv: vec2<f32>, scale: f32) -> f32 {
    let coord = fract(uv / scale - 0.5);
    let dist = vec2<f32>(abs(coord.x - 0.5), abs(coord.y - 0.5));
    let grid = vec2<f32>(
        select(0.0, 1.0, dist.x < 0.5 * dpdx(uv.x / scale)),
        select(0.0, 1.0, dist.y < 0.5 * dpdx(uv.x / scale)),
    );
    // return select(0.0, 1.0, dist.y < 0.5 * dpdx(uv.x / scale));
    let pixel_per_x = dpdx(uv.x);
    let pixel_per_y = dpdx(uv.y);
    let fact = scale / ((pixel_per_x + pixel_per_y)) / 32.0;

    return (grid.x + grid.y) * max(0.0, min(1.0, 1.0)) ;
}


@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {


    let uv = in.coord.xy + 4.0;

    let du_dx = dpdx(uv.x);

    let mixer = 6.0 + log2(du_dx);
    let scale = max(pow(2.0, floor(mixer)), 8.0);
    let  grid_current = grid(uv, scale) * 0.05 + grid(uv, scale * 4.0);
    let  grid_next = grid(uv, scale * 2.0) * 0.05 + grid(uv, scale * 8.0);
    let grid_combined = mix(grid_current, grid_next, fract(mixer));
    // let brightness = max(
    //     1.0 - abs(1.0 - 32.0 * dpdx(coord.x)),
    //     1.0 - abs(1.0 - 64.0 * dpdx(coord.x))
    // ); //max(0.0, abs(1.0 - 1.0 / dpdx(coord.x)));

    return  vec4<f32>(
        vec3(grid_combined),
        //vec3<f32>(grid * 0.5 + 0.5) * brightness,
        1.0
    );
}
