use bevy::{math::vec3, prelude::*};

use bevy_rapier3d::prelude::{Collider, ComputedColliderShape, RigidBody};
use hierarchical_wfc::{
    castle::layout_pass::LayoutTileset,
    graphs::regular_grid_3d,
    materials::tile_pbr_material::TilePbrMaterial,
    tools::MeshBuilder,
    wfc::{
        bevy_passes::{
            WfcEntityMarker, WfcFCollapsedData, WfcInitialData, WfcInvalidatedMarker,
            WfcParentPasses, WfcPassReadyMarker, WfcPendingParentMarker,
        },
        Superposition, TileSet, WfcGraph,
    },
};
use itertools::Itertools;
use rand::{rngs::StdRng, SeedableRng};

use crate::{
    generation::GenerateDebugMarker,
    regenerate::RegenerateSettings,
    replay::{DebugBlocks, ReplayPassProgress, ReplayTileMapMaterials},
};

use super::FacadePassMarker;

#[derive(Component)]
pub struct LayoutPassMarker;

pub fn layout_regenerate_system(
    mut commands: Commands,
    q_regenerating_layouts: Query<
        (
            Entity,
            &WfcFCollapsedData,
            &regular_grid_3d::GraphSettings,
            &regular_grid_3d::GraphData,
            &RegenerateSettings,
        ),
        With<LayoutPassMarker>,
    >,
    q_existing_entities: Query<Entity, (With<WfcEntityMarker>, Without<RegenerateSettings>)>,
) {
    for (
        layout_entity,
        collapsed_data,
        graph_settings,
        graph_data,
        RegenerateSettings { min, max },
    ) in q_regenerating_layouts.iter()
    {
        let min = vec3(2.0, 3.0, 2.0) * *min;
        let max = vec3(2.0, 3.0, 2.0) * *max;

        let tileset = LayoutTileset;

        let graph = WfcGraph {
            nodes: collapsed_data
                .graph
                .nodes
                .iter()
                .enumerate()
                .map(|(i, tile)| {
                    let pos = graph_data.node_positions[i].as_vec3() * graph_settings.spacing;
                    if min.cmple(pos).all() && max.cmpgt(pos).all() {
                        Superposition::filled(tileset.tile_count())
                    } else {
                        Superposition::single(*tile)
                    }
                })
                .collect_vec(),
            neighbours: collapsed_data.graph.neighbours.clone(),
            order: collapsed_data
                .graph
                .order
                .iter()
                .copied()
                .filter(|i| {
                    let pos = graph_data.node_positions[*i].as_vec3() * graph_settings.spacing;
                    !(min.cmplt(pos).all() && max.cmpgt(pos).all())
                })
                .collect_vec(),
        };
        let constraints = tileset.get_constraints();

        let rng = StdRng::from_entropy();

        let mut entity_commands = commands.entity(layout_entity);
        entity_commands.remove::<RegenerateSettings>();
        entity_commands.remove::<WfcFCollapsedData>();
        entity_commands.insert((
            GenerateDebugMarker,
            WfcInitialData {
                label: Some("Layout".to_string()),
                graph,
                constraints,
                weights: tileset.get_weights(),
                rng,
            },
        ));

        for entity in q_existing_entities.iter() {
            commands.entity(entity).insert(WfcInvalidatedMarker);
        }
        commands.spawn((
            WfcEntityMarker,
            FacadePassMarker,
            WfcPendingParentMarker,
            WfcParentPasses(vec![layout_entity]),
        ));
    }
}

type LayoutInitialData = (Entity, &'static regular_grid_3d::GraphSettings);
type LayoutInitialRequired = (With<WfcPassReadyMarker>, With<LayoutPassMarker>);
pub fn layout_init_system(
    mut commands: Commands,
    query: Query<LayoutInitialData, LayoutInitialRequired>,
) {
    for (entity, settings) in query.iter() {
        let tileset = LayoutTileset;

        let (graph_data, wfc_graph) = regular_grid_3d::create_graph(settings, &|(_, _)| {
            Superposition::filled(tileset.tile_count())
        });

        let constraints = tileset.get_constraints();

        let rng = StdRng::from_entropy();

        let mut entity_commands = commands.entity(entity);
        entity_commands.remove::<WfcPassReadyMarker>();
        entity_commands.insert((
            graph_data,
            WfcInitialData {
                label: Some("Layout".to_string()),
                graph: wfc_graph,
                constraints,
                weights: tileset.get_weights(),
                rng,
            },
        ));
    }
}

#[derive(Component)]
pub struct LayoutDebugSettings {
    pub blocks: bool,
    pub arcs: bool,
}

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
        Some(Collider::from_bevy_mesh(&physical_mesh, &ComputedColliderShape::TriMesh).unwrap())
    } else {
        None
    };
    (physical_mesh, non_physical_mesh, physical_mesh_collider)
}

type LayoutCollapsedData = (
    Entity,
    &'static regular_grid_3d::GraphData,
    &'static regular_grid_3d::GraphSettings,
    &'static WfcFCollapsedData,
    &'static LayoutDebugSettings,
);
type LayoutCollapsedRequired = (With<GenerateDebugMarker>, With<LayoutPassMarker>);
pub fn layout_debug_system(
    mut commands: Commands,
    mut q_layout_pass: Query<LayoutCollapsedData, LayoutCollapsedRequired>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut tile_materials: ResMut<Assets<TilePbrMaterial>>,
) {
    for (entity, graph_data, graph_settings, collapsed_data, debug_settings) in
        q_layout_pass.iter_mut()
    {
        dbg!("Creating Debug Mesh");
        commands
            .entity(entity)
            .insert(SpatialBundle::default())
            .insert(ReplayPassProgress {
                length: collapsed_data.graph.order.len(),
                current: collapsed_data.graph.order.len(),
                ..Default::default()
            })
            .remove::<GenerateDebugMarker>();
        if debug_settings.blocks {
            let (solid, air, collider) =
                debug_mesh(&collapsed_data.graph, graph_data, graph_settings);

            let material = tile_materials.add(TilePbrMaterial {
                base_color: Color::rgb(0.6, 0.6, 0.6),
                ..Default::default()
            });

            let mut physics_mesh_commands = commands.spawn((
                WfcEntityMarker,
                MaterialMeshBundle {
                    material: material.clone(),
                    mesh: meshes.add(solid),
                    visibility: Visibility::Visible,
                    ..Default::default()
                },
                DebugBlocks {
                    material_handle: material.clone(),
                },
            ));
            if let Some(collider) = collider {
                physics_mesh_commands.insert((RigidBody::Fixed, collider));
            }
            physics_mesh_commands.set_parent(entity);
            commands
                .spawn((
                    WfcEntityMarker,
                    MaterialMeshBundle {
                        material: material.clone(),
                        mesh: meshes.add(air),
                        visibility: Visibility::Visible,
                        ..Default::default()
                    },
                    DebugBlocks {
                        material_handle: material.clone(),
                    },
                ))
                .set_parent(entity);
            commands
                .entity(entity)
                .insert(ReplayTileMapMaterials(vec![material]));
        }
        if debug_settings.arcs {}
    }
}
