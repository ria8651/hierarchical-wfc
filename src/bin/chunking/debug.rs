use bevy::{math::vec4, prelude::*};
use bevy_rapier3d::prelude::{Collider, ComputedColliderShape, RigidBody};
use hierarchical_wfc::{
    graphs::regular_grid_3d,
    materials::{debug_arc_material::DebugLineMaterial, tile_pbr_material::TilePbrMaterial},
    tools::MeshBuilder,
    wfc::{Neighbour, WfcGraph},
};

use crate::fragments::generate::{ChunkMarker, CollapsedData, FragmentMarker, GenerateDebugMarker};

fn debug_mesh(
    result: &WfcGraph<usize>,
    data: &regular_grid_3d::GraphData,
    settings: &regular_grid_3d::GraphSettings,
) -> (Mesh, Mesh, Option<Collider>) {
    let full_box: Mesh = shape::Box::new(1.9, 2.9, 1.9).into();
    let node_box: Mesh = shape::Cube::new(0.2).into();
    let error_box: Mesh = shape::Cube::new(1.0).into();

    let mut ordering: Vec<usize> = vec![0; result.nodes.len()];
    for (order, index) in result.order.iter().enumerate() {
        ordering[*index] = order;
    }

    let mut physical_mesh_builder = MeshBuilder::new();
    let mut non_physical_mesh_builder = MeshBuilder::new();

    for (index, tile) in result.nodes.iter().enumerate() {
        let position = (data.node_positions[index].as_vec3() + 0.5) * settings.spacing;
        let transform = Transform::from_translation(position);
        let order = ordering[index] as u32;
        match tile {
            0..=3 => physical_mesh_builder.add_mesh(&full_box, transform, order),
            4..=7 => physical_mesh_builder.add_mesh(&full_box, transform, order),
            8 => physical_mesh_builder.add_mesh(&full_box, transform, order),
            9..=12 => non_physical_mesh_builder.add_mesh(&node_box, transform, order),
            13 => non_physical_mesh_builder.add_mesh(&node_box, transform, order),
            404 => physical_mesh_builder.add_mesh(&error_box, transform, order),
            _ => physical_mesh_builder.add_mesh(&error_box, transform, order),
        };
    }
    let physical_mesh = physical_mesh_builder.build();
    let non_physical_mesh = non_physical_mesh_builder.build();
    let physical_mesh_collider = if physical_mesh.count_vertices() > 0 {
        Collider::from_bevy_mesh(&physical_mesh, &ComputedColliderShape::TriMesh)
    } else {
        None
    };
    (physical_mesh, non_physical_mesh, physical_mesh_collider)
}

type LayoutCollapsedData = (
    Entity,
    &'static regular_grid_3d::GraphData,
    &'static regular_grid_3d::GraphSettings,
    &'static CollapsedData,
    &'static Transform,
    Option<&'static FragmentMarker>,
    Option<&'static ChunkMarker>,
);
type LayoutCollapsedRequired = With<GenerateDebugMarker>;
pub fn layout_debug_system(
    mut commands: Commands,
    mut q_layout_pass: Query<LayoutCollapsedData, LayoutCollapsedRequired>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut tile_materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, graph_data, graph_settings, collapsed_data, transform, fragment, chunk) in
        q_layout_pass.iter_mut()
    {
        dbg!("Creating Debug Mesh");
        commands
            .entity(entity)
            .insert(SpatialBundle::default())
            .remove::<GenerateDebugMarker>();
        let (solid, air, collider) = debug_mesh(&collapsed_data.graph, graph_data, graph_settings);

        let material = tile_materials.add(StandardMaterial {
            base_color: match (fragment, chunk) {
                (Some(_), None) => Color::rgb(0.8, 0.6, 0.6),
                (Some(_), None) => Color::rgb(0.6, 0.6, 0.8),
                _ => Color::rgb(0.6, 0.6, 0.8),
            },
            ..Default::default()
        });

        {
            let mut physics_mesh_commands = commands.spawn((MaterialMeshBundle {
                material: material.clone(),
                mesh: meshes.add(solid),
                visibility: Visibility::Visible,
                ..Default::default()
            },));
            physics_mesh_commands.insert(*transform);
            if let Some(collider) = collider {
                physics_mesh_commands.insert((RigidBody::Fixed, collider));
            }
            physics_mesh_commands.set_parent(entity);
        }
        {
            let mut mesh_commands = commands.spawn((MaterialMeshBundle {
                material: material.clone(),
                mesh: meshes.add(air),
                visibility: Visibility::Visible,
                ..Default::default()
            },));
            mesh_commands.insert(*transform);
            mesh_commands.set_parent(entity);
        }
    }
}

pub fn layout_debug_arcs_system(
    mut commands: Commands,
    q_debug_chunks: Query<
        (
            &Transform,
            &CollapsedData,
            &regular_grid_3d::GraphSettings,
            &regular_grid_3d::GraphData,
        ),
        With<GenerateDebugMarker>,
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    mut line_materials: ResMut<Assets<DebugLineMaterial>>,
) {
    for (transform, collapsed_data, graph_settings, graph_data) in q_debug_chunks.iter() {
        let arcs_mesh = create_debug_arcs(&collapsed_data.graph, graph_data, graph_settings);
        commands
            .spawn((
                MaterialMeshBundle {
                    mesh: meshes.add(arcs_mesh),
                    material: line_materials.add(DebugLineMaterial {
                        color: Color::rgb(1.0, 0.0, 1.0),
                    }),
                    visibility: Visibility::Visible,
                    ..Default::default()
                },
                DebugArcs,
            ))
            .insert(transform.clone());
    }
}

const ARC_COLORS: [Vec4; 7] = [
    vec4(1.0, 0.1, 0.1, 1.0),
    vec4(0.1, 1.0, 1.0, 1.0),
    vec4(0.1, 1.0, 0.1, 1.0),
    vec4(1.0, 0.1, 1.0, 1.0),
    vec4(0.1, 0.1, 1.0, 1.0),
    vec4(1.0, 1.0, 0.1, 1.0),
    vec4(0.1, 0.1, 0.1, 1.0),
];

fn create_debug_arcs(
    result: &WfcGraph<usize>,
    data: &regular_grid_3d::GraphData,
    settings: &regular_grid_3d::GraphSettings,
) -> Mesh {
    let mut arc_vertex_positions = Vec::new();
    let mut arc_vertex_normals = Vec::new();
    let mut arc_vertex_uvs = Vec::new();
    let mut arc_vertex_colors = Vec::new();

    for (u, neighbours) in result.neighbours.iter().enumerate() {
        for Neighbour { index: v, arc_type } in neighbours.iter() {
            let color = ARC_COLORS[*arc_type.min(&6)];

            let u = (data.node_positions[u].as_vec3() + 0.5) * settings.spacing;
            let v = (data.node_positions[*v].as_vec3() + 0.5) * settings.spacing;
            let normal = (u - v).normalize();

            arc_vertex_positions.extend([u, v, u, v, v, u]);
            arc_vertex_normals.extend([Vec3::ZERO, Vec3::ZERO, normal, Vec3::ZERO, normal, normal]);

            arc_vertex_uvs.extend([
                Vec2::ZERO,
                (v - u).length() * Vec2::X,
                Vec2::Y,
                (v - u).length() * Vec2::X,
                (v - u).length() * Vec2::X + Vec2::Y,
                Vec2::Y,
            ]);

            arc_vertex_colors.extend([color; 6])
        }
    }

    let mut edges = Mesh::new(bevy::render::render_resource::PrimitiveTopology::TriangleList);
    edges.insert_attribute(Mesh::ATTRIBUTE_POSITION, arc_vertex_positions);
    edges.insert_attribute(Mesh::ATTRIBUTE_NORMAL, arc_vertex_normals);
    edges.insert_attribute(Mesh::ATTRIBUTE_UV_0, arc_vertex_uvs);
    edges.insert_attribute(Mesh::ATTRIBUTE_COLOR, arc_vertex_colors);
    return edges;
}

#[derive(Component)]
struct DebugArcs;
