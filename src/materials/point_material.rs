use bevy::{
    prelude::Color,
    render::render_resource::{AsBindGroup, ShaderRef},
    sprite::Material2d,
};
use bevy_reflect::{TypePath, TypeUuid};

#[derive(AsBindGroup, TypeUuid, TypePath, Debug, Clone)]
#[uuid = "f690fdae-d598-45ab-8225-97e2a3f056e0"]
pub struct PointMaterial {
    #[uniform(0)]
    pub color: Color,
}

impl Material2d for PointMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/tri_point.wgsl".into()
    }
}
