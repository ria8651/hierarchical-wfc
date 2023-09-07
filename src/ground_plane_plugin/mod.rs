use bevy::{
    math::{vec3, vec4},
    prelude::*,
};
use bevy_render::{
    mesh::{Indices, MeshVertexAttribute},
    render_resource::VertexFormat,
    view::NoFrustumCulling,
};

use crate::materials::{
    debug_arc_material::DebugLineMaterial, ground_plane_material::GroundPlaneMaterial,
};
pub struct GroundPlanePlugin;
impl Plugin for GroundPlanePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<GroundPlaneMaterial>::default())
            .add_systems(Startup, ground_plane_init_system);
    }
}

fn ground_plane_init_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GroundPlaneMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let mut edges = Mesh::new(bevy::render::render_resource::PrimitiveTopology::TriangleList);
    edges.insert_attribute(Mesh::ATTRIBUTE_POSITION, vec![vec3(0.0, 0.0, 0.0); 3]);
    edges.set_indices(Some(Indices::U16(vec![0, 1, 2])));

    commands.spawn((
        MaterialMeshBundle {
            mesh: meshes.add(edges),
            transform: Transform::from_xyz(0.0, 0.5, 0.0),
            material: materials.add(GroundPlaneMaterial { color: Color::RED }),
            ..default()
        },
        NoFrustumCulling,
    ));
}
