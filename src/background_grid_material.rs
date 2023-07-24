use bevy::{
    core_pipeline::fullscreen_vertex_shader::{
        fullscreen_shader_vertex_state, FULLSCREEN_SHADER_HANDLE,
    },
    pbr::MaterialPipeline,
    prelude::{dbg, Color, Handle},
    render::{
        mesh::MeshVertexBufferLayout,
        render_resource::{
            AsBindGroup, DepthStencilState, RenderPipelineDescriptor, ShaderRef,
            SpecializedMeshPipelineError,
        },
    },
    sprite::{ColorMaterial, Material2d, Material2dKey},
};
use bevy_reflect::{TypePath, TypeUuid};

#[derive(AsBindGroup, TypeUuid, TypePath, Debug, Clone)]
#[uuid = "076b103b-f3bc-4279-b2d6-f0080b559880"]
pub struct BackgroundGridMaterial {
    #[uniform(0)]
    pub color: Color,
}
impl Material2d for BackgroundGridMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/background_grid.wgsl".into()
    }
    fn vertex_shader() -> ShaderRef {
        "shaders/background_grid.wgsl".into()
    }
}
