use bevy::{asset::ChangeWatcher, gltf::Gltf};
use std::{collections::HashMap, time::Duration};

use bevy::{
    math::{vec3, vec4},
    prelude::{AssetPlugin, PluginGroup, *},
    render::render_resource::{AddressMode, FilterMode, SamplerDescriptor},
};

use bevy::log::LogPlugin;
use bevy_inspector_egui::{bevy_egui, egui, reflect_inspector, DefaultInspectorConfigPlugin};
use bevy_mod_billboard::prelude::*;
use bevy_mod_debugdump;
use bevy_rapier3d::prelude::{
    Collider, ComputedColliderShape, NoUserData, RapierPhysicsPlugin, RigidBody,
};
use hierarchical_wfc::{
    camera_controllers::{
        cam_switcher::{CameraController, SwitchingCameraController, SwitchingCameraPlugin},
        fps::FpsCameraSettings,
    },
    materials::{debug_arc_material::DebugLineMaterial, tile_pbr_material::TilePbrMaterial},
    tools::MeshBuilder,
    village::{
        facade_graph::{FacadePassData, FacadePassSettings, FacadeTileset},
        layout_graph::LayoutGraphSettings,
        layout_pass::LayoutTileset,
    },
    wfc::{
        bevy_passes::{
            wfc_collapse_system, wfc_ready_system, WfcEntityMarker, WfcFCollapsedData,
            WfcInitialData, WfcParentPasses, WfcPassReadyMarker, WfcPendingParentMarker,
        },
        TileSet, WfcGraph,
    },
};
use rand::{rngs::StdRng, SeedableRng};
fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .set(AssetPlugin {
                watch_for_changes: Some(ChangeWatcher {
                    delay: Duration::from_millis(200),
                }),
                ..Default::default()
            })
            .set(ImagePlugin {
                default_sampler: SamplerDescriptor {
                    mag_filter: FilterMode::Nearest,
                    min_filter: FilterMode::Linear,
                    address_mode_u: AddressMode::Repeat,
                    address_mode_v: AddressMode::Repeat,
                    address_mode_w: AddressMode::Repeat,
                    ..Default::default()
                },
            })
            .set(LogPlugin {
                filter: "info,wgpu_core=error,wgpu_hal=error,naga=error,mygame=debug".into(),
                level: bevy::log::Level::DEBUG,
            }),
        SwitchingCameraPlugin,
        RapierPhysicsPlugin::<NoUserData>::default(),
        DefaultInspectorConfigPlugin,
    ))
    .add_plugins(MaterialPlugin::<DebugLineMaterial>::default())
    .add_plugins(MaterialPlugin::<TilePbrMaterial>::default())
    .add_plugins(BillboardPlugin)
    .add_plugins(bevy_egui::EguiPlugin)
    .add_systems(
        Update,
        (
            ui_system,
            wfc_collapse_system,
            wfc_ready_system,
            replay_generation_system,
            layout_init_system,
            // layout_debug_system,
            facade_init_system,
            facade_debug_system,
            facade_mesh_system,
        ),
    )
    .add_systems(Startup, setup);
    #[cfg(not(target_arch = "wasm32"))]
    {
        let settings = bevy_mod_debugdump::render_graph::Settings::default();
        let dot = bevy_mod_debugdump::render_graph_dot(&mut app, &settings);
        std::fs::write("render-graph.dot", dot).expect("Failed to write render-graph.dot");
    }
    app.run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,

    mut ambient_light: ResMut<AmbientLight>,
) {
    let ground_texture = asset_server.load("textures/checker.png");

    let mut ground_mesh =
        Mesh::new(bevy::render::render_resource::PrimitiveTopology::TriangleStrip);
    ground_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0., 1., 0.]; 4]);
    ground_mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[0., 0.], [0.0, 100.], [100., 0.], [100., 100.]],
    );
    ground_mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            vec3(-100.0, 0.0, -100.0),
            vec3(-100.0, 0.0, 100.0),
            vec3(100.0, 0.0, -100.0),
            vec3(100.0, 0.0, 100.0),
        ],
    );

    // plane
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(ground_mesh),

            // shape::Quad::from_size(100f32).into()),
            transform: Transform::from_translation(vec3(0.5, 0.0, 0.5)),
            material: standard_materials.add(StandardMaterial {
                // base_color: Color::rgb(0.3, 0.5, 0.3),`
                base_color_texture: Some(ground_texture),
                perceptual_roughness: 1.0,
                ..Default::default()
            }),
            ..default()
        },
        RigidBody::Fixed,
        Collider::halfspace(Vec3::Y).unwrap(),
    ));

    // light
    commands.spawn((DirectionalLightBundle {
        directional_light: DirectionalLight {
            color: Color::hsl(0.074475f32, 0.15f32, 0.8f32),
            illuminance: 100000f32,
            shadows_enabled: true,
            ..Default::default()
        },

        transform: Transform::IDENTITY.looking_to(vec3(-0.25, -1.0, -0.5), Vec3::Y),
        ..Default::default()
    },));

    ambient_light.color = Color::rgb(0.5, 0.75, 1.0);
    ambient_light.brightness = 0.6;
}

#[derive(Component)]
struct LayoutPass {
    settings: LayoutGraphSettings,
}

fn layout_init_system(
    mut commands: Commands,
    query: Query<(Entity, &LayoutPass), With<WfcPassReadyMarker>>,
) {
    for (entity, LayoutPass { settings }) in query.iter() {
        dbg!("Seeding Layout");

        let tileset = LayoutTileset::default();
        let graph = tileset.create_graph(&settings);
        let constraints = tileset.get_constraints();

        let rng = StdRng::from_entropy();

        let mut entity_commands = commands.entity(entity);
        entity_commands.remove::<WfcPassReadyMarker>();
        entity_commands.insert(WfcInitialData {
            graph,
            constraints,
            weights: tileset.get_weights(),
            rng,
        });
    }
}

#[derive(Component)]
struct PassDebugSettings {
    blocks: bool,
    arcs: bool,
}

#[derive(Component)]
struct GenerateDebugMarker;

fn create_debug_mesh(
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

#[derive(Component)]
struct ReplayPassProgress {
    progress: f32,
    duration: f32,
    playing: bool,
    current: usize,
}
impl Default for ReplayPassProgress {
    fn default() -> Self {
        Self {
            progress: 1.0,
            duration: 2.0,
            playing: false,
            current: 0,
        }
    }
}

fn facade_init_system(
    mut commands: Commands,
    // mut meshes: ResMut<Assets<Mesh>>,
    // mut materials: ResMut<Assets<StandardMaterial>>,
    // mut line_materials: ResMut<Assets<DebugLineMaterial>>,
    query: Query<(Entity, &FacadePassSettings, &WfcParentPasses), With<WfcPassReadyMarker>>,
    q_layout_parents: Query<(&LayoutPass, &WfcFCollapsedData)>,
) {
    for (entity, _pass_settings, parents) in query.iter() {
        for (
            LayoutPass {
                settings: parent_settings,
            },
            parent_data,
        ) in q_layout_parents.iter_many(parents.0.iter())
        {
            let facade_pass_data =
                FacadePassData::from_layout(&parent_data.graph, &parent_settings);

            // // Create debug meshes
            // commands
            //     .spawn((
            //         MaterialMeshBundle {
            //             mesh: meshes
            //                 .add(facade_pass_data.debug_vertex_mesh(shape::Cube::new(0.3).into())),
            //             material: materials.add(Color::rgb(0.8, 0.6, 0.6).into()),
            //             visibility: Visibility::Visible,
            //             ..Default::default()
            //         },
            //         // DebugArcs,
            //     ))
            //     .set_parent(entity);

            // commands
            //     .spawn((MaterialMeshBundle {
            //         mesh: meshes
            //             .add(facade_pass_data.debug_edge_mesh(shape::Cube::new(0.3).into())),
            //         material: materials.add(Color::rgb(0.6, 0.8, 0.6).into()),
            //         visibility: Visibility::Visible,
            //         ..Default::default()
            //     },))
            //     .set_parent(entity);

            // commands
            //     .spawn((MaterialMeshBundle {
            //         mesh: meshes
            //             .add(facade_pass_data.debug_quad_mesh(shape::Cube::new(0.3).into())),
            //         material: materials.add(Color::rgb(0.6, 0.6, 0.8).into()),
            //         visibility: Visibility::Visible,
            //         ..Default::default()
            //     },))
            //     .set_parent(entity);

            // commands
            //     .spawn((
            //         MaterialMeshBundle {
            //             mesh: meshes.add(facade_pass_data.debug_arcs_mesh()),
            //             material: line_materials.add(DebugLineMaterial {
            //                 color: Color::rgb(1.0, 0.0, 1.0),
            //             }),
            //             visibility: Visibility::Visible,
            //             ..Default::default()
            //         },
            //         WfcEntityMarker,
            //     ))
            //     .set_parent(entity);

            let tileset = FacadeTileset::from_asset("semantics/frame_test.json");
            let wfc_graph = facade_pass_data.create_wfc_graph(&tileset);

            let wfc = WfcInitialData {
                graph: wfc_graph,
                constraints: tileset.get_constraints(),
                rng: StdRng::from_entropy(),
                weights: tileset.get_weights(),
            };

            commands
                .entity(entity)
                .remove::<WfcPassReadyMarker>()
                .insert((
                    PassDebugSettings {
                        blocks: false,
                        arcs: false,
                    },
                    GenerateDebugMarker,
                    GenerateMeshMarker,
                ))
                .insert((facade_pass_data, tileset, wfc))
                .insert(SpatialBundle::default());
        }
    }
}
fn replay_generation_system(
    mut q_passes: Query<(&mut ReplayPassProgress, &WfcFCollapsedData, &Children)>,
    q_blocks: Query<&mut DebugBlocks>,
    time: Res<Time>,
    mut tile_materials: ResMut<Assets<TilePbrMaterial>>,
) {
    for (mut progress, collapsed_data, children) in q_passes.iter_mut() {
        progress.current = (collapsed_data.graph.order.len() as f32 * progress.progress) as usize;

        for DebugBlocks { material_handle } in q_blocks.iter_many(children) {
            if let Some(material) = tile_materials.get_mut(&material_handle) {
                material.order_cut_off = progress.current as u32;
            };
        }

        for (_handle, material) in tile_materials.iter_mut() {
            material.order_cut_off = progress.current as u32;
        }

        if progress.playing {
            if progress.progress > 1.0 {
                progress.playing = false;
                progress.progress = 1.0;
            }
            progress.progress += time.delta_seconds() / progress.duration;
        }
    }
}

#[derive(Component)]
struct DebugBlocks {
    material_handle: Handle<TilePbrMaterial>,
}

#[derive(Component)]
struct GenerateMeshMarker;

fn facade_mesh_system(
    mut commands: Commands,
    mut query: Query<
        (Entity, &FacadePassData, &WfcFCollapsedData, &FacadeTileset),
        With<GenerateMeshMarker>,
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, facade_pass_data, collapsed_data, tileset) in query.iter_mut() {
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
                    commands.spawn((
                        SceneBundle {
                            scene: asset_server.load(path),
                            transform,
                            ..Default::default()
                        },
                        WfcEntityMarker,
                    ));
                }
            }
        }

        commands.entity(entity).remove::<GenerateMeshMarker>();
    }
}

fn facade_debug_system(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &FacadePassData,
            &WfcFCollapsedData,
            &FacadeTileset,
            &PassDebugSettings,
        ),
        With<GenerateDebugMarker>,
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut tile_materials: ResMut<Assets<TilePbrMaterial>>,
) {
    let fira_code_handle = asset_server.load("fonts/FiraCode-Bold.ttf");

    for (entity, facade_pass_data, collapsed_data, tileset, debug_settings) in query.iter_mut() {
        if debug_settings.blocks {
            commands
                .entity(entity)
                .insert(ReplayPassProgress::default());

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
                    404 => error_mesh_builder.add_mesh(
                        &error_cube,
                        transform,
                        collapsed_data.graph.order[index] as u32,
                    ),
                    id => {
                        if enable_text {
                            commands.spawn((
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
                                WfcEntityMarker,
                            ));
                        }
                        vertex_mesh_builder.add_mesh(
                            &ok_cube,
                            transform,
                            collapsed_data.graph.order[index] as u32,
                        )
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
                        collapsed_data.graph.order[index] as u32,
                    ),
                    id => {
                        if enable_text {
                            commands.spawn((
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
                                WfcEntityMarker,
                            ));
                        }

                        edge_mesh_builder.add_mesh(
                            &ok_cube,
                            transform,
                            collapsed_data.graph.order[index] as u32,
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
                        collapsed_data.graph.order[index] as u32,
                    ),
                    id => {
                        if enable_text {
                            commands.spawn((
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
                                WfcEntityMarker,
                            ));
                        }
                        quad_mesh_builder.add_mesh(
                            &ok_cube,
                            transform,
                            collapsed_data.graph.order[index] as u32,
                        )
                    }
                }
            }

            // Create debug meshes
            commands
                .spawn((
                    MaterialMeshBundle {
                        mesh: meshes.add(vertex_mesh_builder.build()),
                        material: vertex_material,
                        visibility: Visibility::Visible,
                        ..Default::default()
                    },
                    // DebugArcs,
                ))
                .set_parent(entity);

            commands
                .spawn((
                    MaterialMeshBundle {
                        mesh: meshes.add(edge_mesh_builder.build()),
                        material: edge_material,
                        visibility: Visibility::Visible,
                        ..Default::default()
                    },
                    // DebugArcs,
                ))
                .set_parent(entity);

            commands
                .spawn((
                    MaterialMeshBundle {
                        mesh: meshes.add(quad_mesh_builder.build()),
                        material: quad_material,
                        visibility: Visibility::Visible,
                        ..Default::default()
                    },
                    // DebugArcs,
                ))
                .set_parent(entity);

            commands
                .spawn((
                    MaterialMeshBundle {
                        mesh: meshes.add(error_mesh_builder.build()),
                        material: error_material,
                        visibility: Visibility::Visible,
                        ..Default::default()
                    },
                    // DebugArcs,
                ))
                .set_parent(entity);
        }
        if debug_settings.arcs {}

        commands.entity(entity).remove::<GenerateDebugMarker>();
    }
}

fn layout_debug_system(
    mut commands: Commands,
    mut query: Query<
        (Entity, &LayoutPass, &WfcFCollapsedData, &PassDebugSettings),
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
            .insert(ReplayPassProgress::default())
            .remove::<GenerateDebugMarker>();
        if debug_settings.blocks {
            let (solid, air, collider) =
                create_debug_mesh(&collapsed_data.graph, &layout_pass.settings);

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
        }
        if debug_settings.arcs {}
    }
}

fn ui_system(
    type_registry: ResMut<AppTypeRegistry>,
    mut contexts: bevy_egui::EguiContexts,
    mut commands: Commands,
    mut q_passes: Query<(&mut ReplayPassProgress, &WfcFCollapsedData, &Children)>,
    wfc_entities: Query<Entity, With<WfcEntityMarker>>,
    mut q_cameras: Query<(
        &mut SwitchingCameraController,
        &mut Projection,
        Option<&mut FpsCameraSettings>,
    )>,
    mut layout_settings: Local<LayoutGraphSettings>,
) {
    egui::Window::new("Settings and Controls").show(contexts.ctx_mut(), |ui| {
        ui.collapsing("WFC Settings", |ui| {
            // let mut settings = *layout_settings;
            ui.heading("Settings for layout graph");
            ui.add(egui::DragValue::new(&mut layout_settings.x_size));
            ui.add(egui::DragValue::new(&mut layout_settings.y_size));
            ui.add(egui::DragValue::new(&mut layout_settings.z_size));
            if ui.button("Generate").clicked() {
                for entity in wfc_entities.iter() {
                    commands.entity(entity).despawn_recursive();
                }

                let layout_entity = commands
                    .spawn((
                        WfcEntityMarker,
                        LayoutPass {
                            settings: *layout_settings,
                        },
                        PassDebugSettings {
                            blocks: false,
                            arcs: true,
                        },
                        WfcPassReadyMarker,
                        GenerateDebugMarker,
                    ))
                    .id();

                commands.spawn((
                    WfcEntityMarker,
                    FacadePassSettings,
                    WfcPendingParentMarker,
                    WfcParentPasses(vec![layout_entity]),
                ));
            }
            if ui.button("Clear").clicked() {
                for entity in wfc_entities.iter() {
                    commands.entity(entity).despawn_recursive();
                }
            }
        });

        ui.collapsing("Replay Generation", |ui| {
            for (index, (mut replay_pass, _data, _children)) in q_passes.iter_mut().enumerate() {
                ui.collapsing(format!("{}", index), |ui| {
                    if replay_pass.playing {
                        if ui.button("Pause").clicked() {
                            replay_pass.playing = false;
                        }
                    } else {
                        if ui.button("Play").clicked() {
                            replay_pass.playing = true;
                            if replay_pass.progress >= 1.0 {
                                replay_pass.progress = 0.0;
                            }
                        }
                    }
                    ui.label("Progress");
                    ui.add(
                        egui::Slider::new(&mut replay_pass.progress, 0f32..=1f32).show_value(false),
                    );
                    ui.label("Duration");
                    ui.add(
                        egui::DragValue::new(&mut replay_pass.duration).clamp_range(0f32..=20f32),
                    );
                });
            }
        });

        ui.collapsing("Cameras", |ui| {
            for (mut camera_controller, projection, fps_settings) in q_cameras.iter_mut() {
                egui::ComboBox::from_label("Camera Controller")
                    .selected_text(match camera_controller.selected {
                        CameraController::PanOrbit => "Pan Orbit",
                        CameraController::Fps => "First Person",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut camera_controller.selected,
                            CameraController::PanOrbit,
                            "Pan Orbit",
                        );
                        ui.selectable_value(
                            &mut camera_controller.selected,
                            CameraController::Fps,
                            "First Person",
                        );
                    });
                match camera_controller.selected {
                    CameraController::Fps => {
                        if let Some(mut settings) = fps_settings {
                            reflect_inspector::ui_for_value(
                                settings.as_mut(),
                                ui,
                                &type_registry.read(),
                            );
                        }
                    }
                    CameraController::PanOrbit => {}
                }

                reflect_inspector::ui_for_value(projection.into_inner(), ui, &type_registry.read());
            }
        });
    });
}
