#import bevy_pbr::mesh_bindings   mesh
#import bevy_pbr::mesh_functions  mesh_position_local_to_clip
#import bevy_pbr::mesh_view_bindings view
#import bevy_pbr::mesh_view_bindings globals



struct FullscreenVertexOutput {
    @builtin(position)
    position: vec4<f32>,
    @location(0)
    uv: vec2<f32>,
};

struct DebugLineMaterial {
    color: vec4<f32>,
};
@group(1) @binding(0)
var<uniform> material: DebugLineMaterial;


// This vertex shader produces the following, when drawn using indices 0..3:
//
//  1 |  0-----x.....2
//  0 |  |  s  |  . ´
// -1 |  x_____x´
// -2 |  :  .´
// -3 |  1´
//    +---------------
//      -1  0  1  2  3
//
// The axes are clip-space x and y. The region marked s is the visible region.
// The digits in the corners of the right-angled triangle are the vertex
// indices.
//
// The top-left has UV 0,0, the bottom-left has 0,2, and the top-right has 2,0.
// This means that the UV gets interpolated to 1,1 at the bottom-right corner
// of the clip-space rectangle that is at 1,-1 in clip space.
@vertex
fn vertex(@builtin(vertex_index) vertex_index: u32) -> FullscreenVertexOutput {
    // See the explanation above for how this works
    let uv = vec2<f32>(f32(vertex_index >> 1u), f32(vertex_index & 1u)) * 2.0;
    let clip_position = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);

    return FullscreenVertexOutput(clip_position, uv);
}



struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @builtin(frag_depth) depth: f32,
};


const DEPTH_BIAS: f32 = 1e-5;

@fragment
fn fragment(in: FullscreenVertexOutput) -> FragmentOutput {
    let viewport_coordinate = vec4<f32>( 2.0 * in.uv.x - 1.0, 1.0 - 2.0 * in.uv.y, 0.0, 0.0);
    let ray_dir =  normalize( 
     (
        view.inverse_view_proj * view.projection *  (
         view.inverse_projection * viewport_coordinate
        +   vec4<f32>(0.0, 0.0, -1.0, 0.0)
       )
     ).xyz
    );
 
    // Ray cast from camera to floor 
    let t = -view.world_position.y/ray_dir.y; 
    let floor_pos_world = view.world_position +  t *  ray_dir;
    let floor_pos_clip = view.view_proj *  vec4<f32>(floor_pos_world, 1.0);

    // Render grid    
    var grid_color = vec3<f32>(0.0);
    var grid_opacity = 0.0;
    let target_scale = 16.0 * sqrt( dot(fwidth(floor_pos_world.xz), fwidth(floor_pos_world.xz)));
    let mix_scales  =  pow(fract(log2(target_scale)), 0.5);
    {
        let current_scale = exp2(floor(log2(target_scale)));

        let coord =  floor_pos_world.xz ; 
        let scaled =  coord/ current_scale; 
        let grid = abs(fract(scaled - 0.5 ) - 0.5) / fwidth(coord)*current_scale;
        let line_distance = min(grid.x, grid.y);
        let line = 1.0 - min(line_distance, 1.0);

        grid_opacity += line * 0.25 * (1.0 - mix_scales);
    }
    {
        let next_scale = exp2(ceil(log2(target_scale)));


        let coord =  floor_pos_world.xz ; 
        let scaled =  coord/ next_scale; 
        let grid = abs(fract(scaled - 0.5 ) - 0.5) / fwidth(coord)*next_scale;
        let line_distance = min(grid.x, grid.y);
        let line = 1.0 - min(line_distance, 1.0);

        grid_opacity += line * 0.25 * mix_scales;
    }
    {
        let coord = floor_pos_world.xz; 
        let sdf =  abs(coord);

        var axis =  0.5 * sdf / fwidth(coord);
        let width = 10.0;
        axis = min(1.0 - min(axis, vec2<f32>(1.0)), vec2<f32>(1.0));

        let x_color = vec3<f32>(1.0, 0.2, 0.2);
        let y_color = vec3<f32>(0.2, 0.2, 1.0);

        grid_color = grid_color * (1.0 - axis.x - axis.y) + axis.x * x_color  + axis.y * y_color;
    }
    if t < 0.0 {
            discard;
    }
    var out: FragmentOutput;
    out.color = vec4<f32>(grid_color, grid_opacity);
    out.depth = clamp(floor_pos_clip.z/floor_pos_clip.w - DEPTH_BIAS, 2e-4, 1.0 );
    return out;
}
