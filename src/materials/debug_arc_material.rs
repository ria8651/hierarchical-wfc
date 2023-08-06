use bevy::{
    pbr::Material,
    prelude::Color,
    render::render_resource::{AsBindGroup, ShaderRef},
};
use bevy_reflect::{TypePath, TypeUuid};

#[derive(AsBindGroup, TypeUuid, TypePath, Debug, Clone)]
#[uuid = "076b103b-f3bc-4279-b2d6-f0080b559881"]
pub struct DebugLineMaterial {
    #[uniform(0)]
    pub color: Color,
}
impl Material for DebugLineMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/debug_line.wgsl".into()
    }
    fn vertex_shader() -> ShaderRef {
        "shaders/debug_line.wgsl".into()
    }
    fn alpha_mode(&self) -> bevy::prelude::AlphaMode {
        bevy::prelude::AlphaMode::Blend
    }
}
