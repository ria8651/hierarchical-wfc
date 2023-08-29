use bevy::prelude::*;

use bevy_rapier3d::prelude::{Collider, ComputedColliderShape, RigidBody};
use hierarchical_wfc::{
    materials::tile_pbr_material::TilePbrMaterial,
    tools::MeshBuilder,
    village::{layout_graph::LayoutGraphSettings, layout_pass::LayoutTileset},
    wfc::{
        bevy_passes::{WfcFCollapsedData, WfcInitialData, WfcPassReadyMarker},
        TileSet, WfcGraph,
    },
};
use rand::{rngs::StdRng, SeedableRng};

use crate::{
    generation::GenerateDebugMarker,
    replay::{DebugBlocks, ReplayPassProgress, ReplayTileMapMaterials},
};

#[derive(Component)]
pub struct LayoutPass {
    pub settings: LayoutGraphSettings,
}

pub fn layout_init_system(
    mut commands: Commands,
    query: Query<(Entity, &LayoutPass), With<WfcPassReadyMarker>>,
) {
    for (entity, LayoutPass { settings }) in query.iter() {
        dbg!("Seeding Layout");

        let tileset = LayoutTileset;
        let graph = tileset.create_graph(settings);
        let constraints = tileset.get_constraints();

        let rng = StdRng::from_entropy();

        let mut entity_commands = commands.entity(entity);
        entity_commands.remove::<WfcPassReadyMarker>();
        entity_commands.insert(WfcInitialData {
            label: Some("Layout".to_string()),
            graph,
            constraints,
            weights: tileset.get_weights(),
            rng,
        });
    }
}

#[derive(Component)]
pub struct LayoutDebugSettings {
    pub blocks: bool,
    pub arcs: bool,
}

fn debug_mesh(
    result: &WfcGraph<usize>,
    settings: &LayoutGraphSettings,
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
        let position = settings.posf32_from_index(index);
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

pub fn layout_debug_system(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &LayoutPass,
            &WfcFCollapsedData,
            &LayoutDebugSettings,
        ),
        With<GenerateDebugMarker>,
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    mut tile_materials: ResMut<Assets<TilePbrMaterial>>,
) {
    for (entity, layout_pass, collapsed_data, debug_settings) in query.iter_mut() {
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
            let (solid, air, collider) = debug_mesh(&collapsed_data.graph, &layout_pass.settings);

            let material = tile_materials.add(TilePbrMaterial {
                base_color: Color::rgb(0.6, 0.6, 0.6),
                ..Default::default()
            });

            let mut physics_mesh_commands = commands.spawn((
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
                .insert(ReplayTileMapMaterials { 0: vec![material] });
        }
        if debug_settings.arcs {}
    }
}
