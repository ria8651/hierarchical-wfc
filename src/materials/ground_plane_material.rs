use bevy::{
    core_pipeline::fullscreen_vertex_shader::FULLSCREEN_SHADER_HANDLE,
    pbr::Material,
    prelude::Color,
    render::render_resource::{AsBindGroup, ShaderRef},
};
use bevy_reflect::{TypePath, TypeUuid};
use bevy_render::{
    mesh::{Mesh, MeshVertexAttribute},
    render_resource::VertexFormat,
};

#[derive(AsBindGroup, TypeUuid, TypePath, Debug, Clone)]
#[uuid = "dae963b7-0c92-4542-9343-345c07f7909b"]
pub struct GroundPlaneMaterial {
    #[uniform(0)]
    pub color: Color,
}
impl Material for GroundPlaneMaterial {
    fn specialize(
        pipeline: &bevy::pbr::MaterialPipeline<Self>,
        descriptor: &mut bevy_render::render_resource::RenderPipelineDescriptor,
        layout: &bevy_render::mesh::MeshVertexBufferLayout,
        key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), bevy_render::render_resource::SpecializedMeshPipelineError> {
        let vertex_layout = layout.get_layout(&[Mesh::ATTRIBUTE_POSITION.at_shader_location(0)])?;
        dbg!(&descriptor.vertex.buffers);

        descriptor.vertex.buffers = vec![vertex_layout];

        Ok(())
    }
    fn fragment_shader() -> ShaderRef {
        "shaders/ground_plane.wgsl".into()
    }
    fn vertex_shader() -> ShaderRef {
        "shaders/ground_plane.wgsl".into()
    }
    fn alpha_mode(&self) -> bevy::prelude::AlphaMode {
        bevy::prelude::AlphaMode::Blend
    }
}
