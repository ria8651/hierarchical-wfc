use bevy::{math::vec4, prelude::*, utils::HashMap};
use bevy_rapier3d::prelude::{Collider, ComputedColliderShape};
use hierarchical_wfc::{
    graphs::regular_grid_3d,
    materials::debug_arc_material::DebugLineMaterial,
    tools::MeshBuilder,
    wfc::{Neighbour, WfcGraph},
};

use crate::fragments::{
    generate::FragmentLocation,
    plugin::{
        ChunkLoadEvent, ChunkMarker, CollapsedData, GenerateDebugMarker, GenerationDebugSettings,
    },
    systems::AsyncWorld,
};

pub fn debug_mesh(
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

pub fn layout_debug_visibility_system(
    debug_settings: Res<GenerationDebugSettings>,
    mut q_fragments: Query<(&FragmentLocation, &mut Visibility)>,
) {
    if debug_settings.is_changed() {
        for (frag_type, mut visibility) in q_fragments.iter_mut() {
            let show = match frag_type {
                FragmentLocation::Node(..) => debug_settings.show_fragment_nodes,
                FragmentLocation::Edge(..) => debug_settings.show_fragment_edges,
                FragmentLocation::Face(..) => debug_settings.show_fragment_faces,
            };
            *visibility = match show {
                true => Visibility::Visible,
                false => Visibility::Hidden,
            };
        }
    }
}

pub fn layout_debug_reset_system(
    mut commands: Commands,
    mut ev_chunk_load: EventReader<ChunkLoadEvent>,
    q_fragments: Query<Entity, With<FragmentLocation>>,
    q_chunks: Query<Entity, With<ChunkMarker>>,
) {
    for ev in ev_chunk_load.iter() {
        match ev {
            ChunkLoadEvent::Reset => {
                for entity in q_fragments.iter() {
                    commands.entity(entity).despawn_recursive();
                }
                for entity in q_chunks.iter() {
                    commands.entity(entity).despawn_recursive();
                }
            }
            _ => {}
        }
    }
}

#[derive(Default, Resource)]
pub struct LoadedFragments {
    loaded: HashMap<FragmentLocation, Entity>,
}

#[derive(Default, Resource)]
pub struct LoadedChunks {
    fragments: HashMap<IVec3, Vec<FragmentLocation>>,
}

pub fn fragment_debug_instantiation_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut tile_materials: ResMut<Assets<StandardMaterial>>,
    mut async_world: ResMut<AsyncWorld>,
    mut loaded_fragments: ResMut<LoadedFragments>,
    debug_settings: Res<GenerationDebugSettings>,
) {
    // Process only one event per frame otherwise bevy will freeze while preparing lots of meshes at the same time
    if let Ok(event) = async_world.rx_fragment_instantiate.try_recv() {
        let (create, show) = match event.fragment_location {
            FragmentLocation::Node(..) => (
                debug_settings.create_fragment_nodes,
                debug_settings.show_fragment_nodes,
            ),
            FragmentLocation::Edge(..) => (
                debug_settings.create_fragment_edges,
                debug_settings.show_fragment_edges,
            ),
            FragmentLocation::Face(..) => (
                debug_settings.create_fragment_faces,
                debug_settings.show_fragment_faces,
            ),
        };
        if !create {
            return;
        }

        let visibility = match show {
            true => Visibility::Visible,
            false => Visibility::Hidden,
        };

        let entity = commands
            .spawn((
                event.fragment_location.clone(),
                event.collapsed,
                event.data,
                event.settings,
                SpatialBundle {
                    visibility,
                    transform: event.transform,
                    ..Default::default()
                },
            ))
            .id();

        let (solid, air, _collider) = event.meshes;
        let material = tile_materials.add(StandardMaterial {
            base_color: match event.fragment_location {
                FragmentLocation::Node(..) => Color::rgb(0.8, 0.6, 0.6),
                FragmentLocation::Edge(..) => Color::rgb(0.6, 0.8, 0.6),
                FragmentLocation::Face(..) => Color::rgb(0.6, 0.6, 0.8),
            },
            ..Default::default()
        });

        {
            let mut physics_mesh_commands = commands.spawn(MaterialMeshBundle {
                material: material.clone(),
                mesh: meshes.add(solid.clone()),
                visibility: Visibility::Inherited,
                ..Default::default()
            });
            // if let Some(collider) = collider {
            //     physics_mesh_commands.insert((RigidBody::Fixed, collider));
            // }
            physics_mesh_commands.set_parent(entity);
        }
        {
            let mut mesh_commands = commands.spawn(MaterialMeshBundle {
                material: material.clone(),
                mesh: meshes.add(air),
                visibility: Visibility::Inherited,
                ..Default::default()
            });
            mesh_commands.set_parent(entity);
        }
        if let Some(old) = loaded_fragments
            .loaded
            .insert(event.fragment_location, entity)
        {
            // Old chunk wasn't despawned yet !!! WON"T WORK!
            // commands.entity(old).despawn_recursive();
        }
    }
}

pub fn fragment_debug_destruction_system(
    mut commands: Commands,
    mut async_world: ResMut<AsyncWorld>,
    mut loaded_fragments: ResMut<LoadedFragments>,
) {
    // Process only one event per frame otherwise bevy will freeze while preparing lots of meshes at the same time
    if let Ok(event) = async_world.rx_fragment_destroy.try_recv() {
        if let Some(fragment) = loaded_fragments.loaded.remove(&event.fragment_location) {
            commands.entity(fragment).despawn_recursive();
        }
    }
}

pub fn _layout_debug_arcs_system(
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
        let arcs_mesh = _create_debug_arcs(&collapsed_data.graph, graph_data, graph_settings);
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
            .insert(*transform);
    }
}

const _ARC_COLORS: [Vec4; 7] = [
    vec4(1.0, 0.1, 0.1, 1.0),
    vec4(0.1, 1.0, 1.0, 1.0),
    vec4(0.1, 1.0, 0.1, 1.0),
    vec4(1.0, 0.1, 1.0, 1.0),
    vec4(0.1, 0.1, 1.0, 1.0),
    vec4(1.0, 1.0, 0.1, 1.0),
    vec4(0.1, 0.1, 0.1, 1.0),
];

fn _create_debug_arcs(
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
            let color = _ARC_COLORS[*arc_type.min(&6)];

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
    edges
}

#[derive(Component)]
struct DebugArcs;
