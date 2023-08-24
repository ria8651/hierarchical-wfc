#import bevy_pbr::prepass_bindings
#import bevy_pbr::mesh_functions
#import bevy_pbr::skinning
#import bevy_pbr::morph
#import bevy_pbr::mesh_bindings mesh

@group(1) @binding(0)
var<uniform> material: TilePbrMaterial;

struct TilePbrMaterial {
    base_color: vec4<f32>,
    emissive: vec4<f32>,
    perceptual_roughness: f32,
    metallic: f32,
    reflectance: f32,
    // 'flags' is a bit field indicating various options. u32 is 32 bits so we have up to 32 options.
    flags: u32,
    alpha_cutoff: f32,
    parallax_depth_scale: f32,
    max_parallax_layer_count: f32,
    max_relief_mapping_search_steps: u32,
    order_cut_off: u32,
};


// Most of these attributes are not used in the default prepass fragment shader, but they are still needed so we can
// pass them to custom prepass shaders like pbr_prepass.wgsl.
struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) order: u32
    
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,

#ifdef VERTEX_UVS
    @location(0) uv: vec2<f32>,
#endif // VERTEX_UVS

#ifdef NORMAL_PREPASS
    @location(1) world_normal: vec3<f32>,
#ifdef VERTEX_TANGENTS
    @location(2) world_tangent: vec4<f32>,
#endif // VERTEX_TANGENTS
#endif // NORMAL_PREPASS

#ifdef MOTION_VECTOR_PREPASS
    @location(3) world_position: vec4<f32>,
    @location(4) previous_world_position: vec4<f32>,
#endif // MOTION_VECTOR_PREPASS

#ifdef DEPTH_CLAMP_ORTHO
    @location(5) clip_position_unclamped: vec4<f32>,
#endif // DEPTH_CLAMP_ORTHO
}

#ifdef MORPH_TARGETS
fn morph_vertex(vertex_in: Vertex) -> Vertex {
    var vertex = vertex_in;
    let weight_count = bevy_pbr::morph::layer_count();
    for (var i: u32 = 0u; i < weight_count; i ++) {
        let weight = bevy_pbr::morph::weight_at(i);
        if weight == 0.0 {
            continue;
        }
        vertex.position += weight * bevy_pbr::morph::morph(vertex.index, bevy_pbr,:: morph,:: position_offset, i);
#ifdef VERTEX_NORMALS
        vertex.normal += weight * bevy_pbr::morph::morph(vertex.index, bevy_pbr,:: morph,:: normal_offset, i);
#endif
#ifdef VERTEX_TANGENTS
        vertex.tangent += vec4(weight * bevy_pbr,:: morph,:: morph(vertex.index, bevy_pbr,:: morph,:: tangent_offset, i), 0.0);
#endif
    }
    return vertex;
}
#endif

@vertex
fn vertex(vertex_no_morph: Vertex) -> VertexOutput {
    var out: VertexOutput;




#ifdef MORPH_TARGETS
    var vertex = morph_vertex(vertex_no_morph);
#else
    var vertex = vertex_no_morph;
#endif

#ifdef SKINNED
    var model = bevy_pbr::skinning::skin_model(vertex.joint_indices, vertex.joint_weights);
#else // SKINNED
    // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
    // See https://github.com/gfx-rs/naga/issues/2416
    var model = mesh.model;
#endif // SKINNED

    if vertex.order >= material.order_cut_off {
        out.clip_position = vec4<f32>(2.0);
        return out;
    }
    out.clip_position = bevy_pbr::mesh_functions::mesh_position_local_to_clip(model, vec4(vertex.position, 1.0));


#ifdef DEPTH_CLAMP_ORTHO
    out.clip_position_unclamped = out.clip_position;
    out.clip_position.z = min(out.clip_position.z, 1.0);
#endif // DEPTH_CLAMP_ORTHO

#ifdef VERTEX_UVS
    out.uv = vertex.uv;
#endif // VERTEX_UVS

#ifdef NORMAL_PREPASS
#ifdef SKINNED
    out.world_normal = bevy_pbr::skinning::skin_normals(model, vertex.normal);
#else // SKINNED
    out.world_normal = vertex.normal;
       
#endif // SKINNED

#ifdef VERTEX_TANGENTS
    out.world_tangent = bevy_pbr::mesh_functions::mesh_tangent_local_to_world(
        model,
        vertex.tangent,
        // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
        // See https://github.com/gfx-rs/naga/issues/2416
        vertex_no_morph.instance_index
    );
#endif // VERTEX_TANGENTS
#endif // NORMAL_PREPASS

#ifdef MOTION_VECTOR_PREPASS
    out.world_position = bevy_pbr::mesh_functions::mesh_position_local_to_world(model, vec4<f32>(vertex.position, 1.0));
    // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
    // See https://github.com/gfx-rs/naga/issues/2416
    out.previous_world_position = bevy_pbr::mesh_functions::mesh_position_local_to_world(
        mesh.previous_model,
        vec4<f32>(vertex.position, 1.0)
    );
#endif // MOTION_VECTOR_PREPASS

    return out;
}

// #ifdef PREPASS_FRAGMENT
// struct FragmentInput {
// #ifdef VERTEX_UVS
//     @location(0) uv: vec2<f32>,
// #endif // VERTEX_UVS

// #ifdef NORMAL_PREPASS
//     @location(1) world_normal: vec3<f32>,
// #endif // NORMAL_PREPASS

// #ifdef MOTION_VECTOR_PREPASS
//     @location(3) world_position: vec4<f32>,
//     @location(4) previous_world_position: vec4<f32>,
// #endif // MOTION_VECTOR_PREPASS

// #ifdef DEPTH_CLAMP_ORTHO
//     @location(5) clip_position_unclamped: vec4<f32>,
// #endif // DEPTH_CLAMP_ORTHO
// }

// struct FragmentOutput {
// #ifdef NORMAL_PREPASS
//     @location(0) normal: vec4<f32>,
// #endif // NORMAL_PREPASS

// #ifdef MOTION_VECTOR_PREPASS
//     @location(1) motion_vector: vec2<f32>,
// #endif // MOTION_VECTOR_PREPASS

// #ifdef DEPTH_CLAMP_ORTHO
//     @builtin(frag_depth) frag_depth: f32,
// #endif // DEPTH_CLAMP_ORTHO
// }

// @fragment
// fn fragment(in: FragmentInput) -> FragmentOutput {
//     var out: FragmentOutput;

// #ifdef NORMAL_PREPASS
//     out.normal = vec4(in.world_normal * 0.5 + vec3(0.5), 1.0);
// #endif

// #ifdef DEPTH_CLAMP_ORTHO
//     out.frag_depth = in.clip_position_unclamped.z;
// #endif // DEPTH_CLAMP_ORTHO

// #ifdef MOTION_VECTOR_PREPASS
//     let clip_position_t = bevy_pbr::prepass_bindings::view.unjittered_view_proj * in.world_position;
//     let clip_position = clip_position_t.xy / clip_position_t.w;
//     let previous_clip_position_t = bevy_pbr::prepass_bindings::previous_view_proj * in.previous_world_position;
//     let previous_clip_position = previous_clip_position_t.xy / previous_clip_position_t.w;
//     // These motion vectors are used as offsets to UV positions and are stored
//     // in the range -1,1 to allow offsetting from the one corner to the
//     // diagonally-opposite corner in UV coordinates, in either direction.
//     // A difference between diagonally-opposite corners of clip space is in the
//     // range -2,2, so this needs to be scaled by 0.5. And the V direction goes
//     // down where clip space y goes up, so y needs to be flipped.
//     out.motion_vector = (clip_position - previous_clip_position) * vec2(0.5, -0.5);
// #endif // MOTION_VECTOR_PREPASS

//     return out;
// }
// #endif // PREPASS_FRAGMENT