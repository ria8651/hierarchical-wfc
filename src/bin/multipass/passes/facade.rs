use bevy::{math::vec3, prelude::*};

use bevy_mod_billboard::prelude::*;

use hierarchical_wfc::{
    castle::facade_graph::{FacadePassData, FacadePassSettings, FacadeTileset},
    graphs::regular_grid_3d,
    materials::tile_pbr_material::TilePbrMaterial,
    tools::MeshBuilder,
    wfc::{
        bevy_passes::{
            WfcEntityMarker, WfcFCollapsedData, WfcInitialData, WfcParentPasses, WfcPassReadyMarker,
        },
        TileSet,
    },
};
use rand::{rngs::StdRng, SeedableRng};

use crate::{
    generation::GenerateDebugMarker,
    replay::{ReplayOrder, ReplayPassProgress, ReplayTileMapMaterials},
};

use super::LayoutPassMarker;

#[derive(Component)]
pub struct FacadeDebugSettings {
    blocks: bool,
}

pub fn facade_init_system(
    mut commands: Commands,
    query: Query<(Entity, &FacadePassSettings, &WfcParentPasses), With<WfcPassReadyMarker>>,
    q_layout_parents: Query<
        (&regular_grid_3d::GraphSettings, &WfcFCollapsedData),
        With<LayoutPassMarker>,
    >,
) {
    for (entity, _pass_settings, parents) in query.iter() {
        for (graph_settings, collapsed_data) in q_layout_parents.iter_many(parents.0.iter()) {
            let facade_pass_data =
                FacadePassData::from_layout(graph_settings, &collapsed_data.graph);

            let tileset = FacadeTileset::from_asset("semantics/frame_test.json");
            let wfc_graph = facade_pass_data.create_wfc_graph(&tileset);

            let wfc = WfcInitialData {
                label: Some("Facade".to_string()),
                graph: wfc_graph,
                constraints: tileset.get_constraints(),
                rng: StdRng::from_entropy(),
                weights: tileset.get_weights(),
            };

            commands
                .entity(entity)
                .remove::<WfcPassReadyMarker>()
                .insert((
                    FacadeDebugSettings { blocks: true },
                    GenerateDebugMarker,
                    GenerateMeshMarker,
                    WfcEntityMarker,
                ))
                .insert((facade_pass_data, tileset, wfc))
                .insert(SpatialBundle::default());
        }
    }
}

#[derive(Component)]
pub struct GenerateMeshMarker;

pub fn facade_mesh_system(
    mut commands: Commands,
    mut query: Query<
        (Entity, &FacadePassData, &WfcFCollapsedData, &FacadeTileset),
        With<GenerateMeshMarker>,
    >,
    asset_server: Res<AssetServer>,
) {
    for (entity, facade_pass_data, collapsed_data, tileset) in query.iter_mut() {
        let mut ordering: Vec<usize> = vec![0; collapsed_data.graph.nodes.len()];
        for (order, index) in collapsed_data.graph.order.iter().enumerate() {
            ordering[*index] = order;
        }
        for (node_index, node) in collapsed_data.graph.nodes.iter().enumerate() {
            let node = *node;

            // let node = collapsed_data.graph.nodes
            //     [edge_id + facade_pass_data.vertices.len() + facade_pass_data.edges.len()];

            if node != 404 {
                let transformed_id = tileset.leaf_sources[node];
                let transformed_node = &tileset.transformed_nodes[transformed_id];
                let symmetry = &transformed_node.symmetry;
                const DIRECTIONS: [Vec4; 6] = [
                    Vec4::X,
                    Vec4::NEG_X,
                    Vec4::Y,
                    Vec4::NEG_Y,
                    Vec4::Z,
                    Vec4::NEG_Z,
                ];

                let position = facade_pass_data.get_node_pos(node_index);
                // let position = edge.pos.as_vec3() * vec3(2.0, 3.0, 2.0) * 0.25;
                let transform = Transform::from_matrix(
                    Mat4::from_cols(
                        DIRECTIONS[symmetry[0]],
                        DIRECTIONS[symmetry[2]],
                        DIRECTIONS[symmetry[4]],
                        Vec4::W,
                    )
                    .inverse(),
                )
                .with_translation(position);

                let models = tileset.assets.get("models").unwrap();
                // TODO: Less indirection here!
                if let Some(model) = &models.nodes[transformed_node.source_node] {
                    let path = format!("{}/{}", models.path, model);
                    commands
                        .spawn((
                            SceneBundle {
                                scene: asset_server.load(path),
                                transform,
                                ..Default::default()
                            },
                            ReplayOrder(ordering[node_index]),
                        ))
                        .set_parent(entity);
                }
            }
        }

        commands.entity(entity).remove::<GenerateMeshMarker>();
    }
}

pub fn facade_debug_system(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &FacadePassData,
            &WfcFCollapsedData,
            &FacadeTileset,
            &FacadeDebugSettings,
        ),
        With<GenerateDebugMarker>,
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    asset_server: Res<AssetServer>,
    mut tile_materials: ResMut<Assets<TilePbrMaterial>>,
) {
    let fira_code_handle = asset_server.load("fonts/FiraCode-Bold.ttf");

    for (entity, facade_pass_data, collapsed_data, tileset, debug_settings) in query.iter_mut() {
        let mut ordering: Vec<usize> = vec![0; collapsed_data.graph.nodes.len()];
        for (order, index) in collapsed_data.graph.order.iter().enumerate() {
            ordering[*index] = order;
        }
        if debug_settings.blocks {
            commands.entity(entity).insert(ReplayPassProgress {
                length: collapsed_data.graph.order.len(),
                current: collapsed_data.graph.order.len(),
                ..Default::default()
            });

            let enable_text = false;

            let ok_cube: Mesh = shape::Cube::new(0.25).into();
            let error_cube: Mesh = shape::Cube::new(0.5).into();

            let mut vertex_mesh_builder = MeshBuilder::new();
            let mut edge_mesh_builder = MeshBuilder::new();
            let mut quad_mesh_builder = MeshBuilder::new();
            let mut error_mesh_builder = MeshBuilder::new();

            let vertex_material = tile_materials.add(TilePbrMaterial {
                base_color: Color::rgb(0.8, 0.6, 0.6),
                ..Default::default()
            });

            let edge_material = tile_materials.add(TilePbrMaterial {
                base_color: Color::rgb(0.6, 0.8, 0.6),
                ..Default::default()
            });

            let quad_material = tile_materials.add(TilePbrMaterial {
                base_color: Color::rgb(0.6, 0.6, 0.8),
                ..Default::default()
            });

            let error_material = tile_materials.add(TilePbrMaterial {
                base_color: Color::rgb(0.9, 0.2, 0.2),
                ..Default::default()
            });

            for (index, vert) in facade_pass_data.vertices.iter().enumerate() {
                let transform =
                    Transform::from_translation(vert.pos.as_vec3() * vec3(2.0, 3.0, 2.0));
                match collapsed_data.graph.nodes[index] {
                    404 => {
                        error_mesh_builder.add_mesh(&error_cube, transform, ordering[index] as u32)
                    }
                    id => {
                        if enable_text {
                            commands.spawn((
                                WfcEntityMarker,
                                BillboardTextBundle {
                                    transform: transform
                                        .with_scale(Vec3::ONE * 0.0025)
                                        .with_translation(transform.translation + 0.25 * Vec3::Y),
                                    text: Text::from_sections([TextSection {
                                        value: format!(
                                            "{} [{}]",
                                            tileset.get_leaf_semantic_name(id),
                                            id
                                        ),
                                        style: TextStyle {
                                            font_size: 60.0,
                                            font: fira_code_handle.clone(),
                                            color: Color::rgb(0.9, 0.4, 0.4),
                                        },
                                    }])
                                    .with_alignment(TextAlignment::Center),
                                    ..default()
                                },
                            ));
                        }
                        vertex_mesh_builder.add_mesh(&ok_cube, transform, ordering[index] as u32)
                    }
                }
            }
            for (index, edge) in facade_pass_data.edges.iter().enumerate() {
                let transform =
                    Transform::from_translation(edge.pos.as_vec3() * vec3(2.0, 3.0, 2.0) * 0.5);
                match collapsed_data.graph.nodes[index + facade_pass_data.vertices.len()] {
                    404 => error_mesh_builder.add_mesh(
                        &error_cube,
                        transform,
                        ordering[index + facade_pass_data.vertices.len()] as u32,
                    ),
                    id => {
                        if enable_text {
                            commands.spawn((
                                WfcEntityMarker,
                                BillboardTextBundle {
                                    transform: transform
                                        .with_scale(Vec3::ONE * 0.0025)
                                        .with_translation(transform.translation + 0.25 * Vec3::Y),
                                    text: Text::from_sections([TextSection {
                                        value: format!(
                                            "{} [{}]",
                                            tileset.get_leaf_semantic_name(id),
                                            id
                                        ),
                                        style: TextStyle {
                                            font_size: 60.0,
                                            font: fira_code_handle.clone(),
                                            color: Color::rgb(0.4, 0.9, 0.4),
                                        },
                                    }])
                                    .with_alignment(TextAlignment::Center),
                                    ..default()
                                },
                            ));
                        }

                        edge_mesh_builder.add_mesh(
                            &ok_cube,
                            transform,
                            ordering[index + facade_pass_data.vertices.len()] as u32,
                        )
                    }
                }
            }
            for (index, quad) in facade_pass_data.quads.iter().enumerate() {
                let transform =
                    Transform::from_translation(quad.pos.as_vec3() * vec3(2.0, 3.0, 2.0) * 0.25);
                match collapsed_data.graph.nodes
                    [index + facade_pass_data.vertices.len() + facade_pass_data.edges.len()]
                {
                    404 => error_mesh_builder.add_mesh(
                        &error_cube,
                        transform,
                        ordering
                            [index + facade_pass_data.vertices.len() + facade_pass_data.edges.len()]
                            as u32,
                    ),
                    id => {
                        if enable_text {
                            commands.spawn((
                                WfcEntityMarker,
                                BillboardTextBundle {
                                    transform: transform
                                        .with_scale(Vec3::ONE * 0.0025)
                                        .with_translation(transform.translation + 0.25 * Vec3::Y),
                                    text: Text::from_sections([TextSection {
                                        value: format!(
                                            "{} [{}]",
                                            tileset.get_leaf_semantic_name(id),
                                            id
                                        ),
                                        style: TextStyle {
                                            font_size: 60.0,
                                            font: fira_code_handle.clone(),
                                            color: Color::rgb(0.4, 0.4, 0.9),
                                        },
                                    }])
                                    .with_alignment(TextAlignment::Center),
                                    ..default()
                                },
                            ));
                        }
                        quad_mesh_builder.add_mesh(
                            &ok_cube,
                            transform,
                            ordering[index
                                + facade_pass_data.vertices.len()
                                + facade_pass_data.edges.len()] as u32,
                        )
                    }
                }
            }

            // Create debug meshes
            commands
                .spawn((MaterialMeshBundle {
                    mesh: meshes.add(vertex_mesh_builder.build()),
                    material: vertex_material.clone(),
                    visibility: Visibility::Visible,
                    ..Default::default()
                },))
                .set_parent(entity);

            commands
                .spawn((MaterialMeshBundle {
                    mesh: meshes.add(edge_mesh_builder.build()),
                    material: edge_material.clone(),
                    visibility: Visibility::Visible,
                    ..Default::default()
                },))
                .set_parent(entity);

            commands
                .spawn((MaterialMeshBundle {
                    mesh: meshes.add(quad_mesh_builder.build()),
                    material: quad_material.clone(),
                    visibility: Visibility::Visible,
                    ..Default::default()
                },))
                .set_parent(entity);

            commands
                .spawn((MaterialMeshBundle {
                    mesh: meshes.add(error_mesh_builder.build()),
                    material: error_material.clone(),
                    visibility: Visibility::Visible,
                    ..Default::default()
                },))
                .set_parent(entity);
            commands.entity(entity).insert(ReplayTileMapMaterials(vec![
                error_material,
                vertex_material,
                edge_material,
                quad_material,
            ]));
        }

        commands.entity(entity).remove::<GenerateDebugMarker>();
    }
}
