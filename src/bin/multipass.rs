use bevy::{asset::ChangeWatcher, ecs::system::SystemState, window::PresentMode};
use bevy_render::{render_resource::TextureDescriptor, texture::ImageSampler};
use std::time::Duration;

use bevy::{
    math::vec3,
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
    camera_plugin::{
        cam_switcher::{CameraController, SwitchingCameraController, SwitchingCameraPlugin},
        fps::FpsCameraSettings,
    },
    materials::{debug_arc_material::DebugLineMaterial, tile_pbr_material::TilePbrMaterial},
    tools::MeshBuilder,
    ui_plugin::{EcsUiNode, EcsUiPlugin, EguiWindow, UiState},
    village::{
        facade_graph::{FacadePassData, FacadePassSettings, FacadeTileset},
        layout_graph::LayoutGraphSettings,
        layout_pass::LayoutTileset,
    },
    wfc::{
        bevy_passes::{
            wfc_collapse_system, wfc_ready_system, WfcEntityMarker, WfcFCollapsedData,
            WfcInitialData, WfcInvalidatedMarker, WfcParentPasses, WfcPassReadyMarker,
            WfcPendingParentMarker,
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
                    mag_filter: FilterMode::Linear,
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
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: PresentMode::Fifo,
                    ..Default::default()
                }),
                ..Default::default()
            }),
        SwitchingCameraPlugin,
        RapierPhysicsPlugin::<NoUserData>::default(),
        DefaultInspectorConfigPlugin,
        MaterialPlugin::<DebugLineMaterial>::default(),
        MaterialPlugin::<TilePbrMaterial>::default(),
        BillboardPlugin,
        bevy_egui::EguiPlugin,
        EcsUiPlugin,
    ))
    .add_systems(
        Update,
        (
            wfc_collapse_system,
            wfc_ready_system,
            replay_generation_system,
            layout_init_system,
            layout_debug_system,
            facade_init_system,
            facade_debug_system,
            facade_mesh_system,
            set_ground_sampler,
        ),
    )
    .add_systems(Startup, (setup, init_inspector))
    .add_systems(PostUpdate, wfc_despawn_invalid_system);
    #[cfg(not(target_arch = "wasm32"))]
    {
        let settings = bevy_mod_debugdump::render_graph::Settings::default();
        let dot = bevy_mod_debugdump::render_graph_dot(&mut app, &settings);
        std::fs::write("render-graph.dot", dot).expect("Failed to write render-graph.dot");
    }
    app.run();
}

fn wfc_despawn_invalid_system(
    mut commands: Commands,
    q_invalid: Query<Entity, With<WfcInvalidatedMarker>>,
) {
    for entity in q_invalid.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

#[derive(SystemSet, Hash, Debug, Eq, PartialEq, Clone, Component)]
struct GroundPlane;

fn init_inspector(world: &mut World) {
    let layout_settings_tab: Box<dyn EcsUiNode + Send + Sync> = EcsUiLayout::new(world);
    let cameras_tab: Box<dyn EcsUiNode + Send + Sync> = EcsUiCameras::new(world);
    let replay_tab: Box<dyn EcsUiNode + Send + Sync> = EcsUiReplay::new(world);

    let mut tree = egui_dock::Tree::new(vec![EguiWindow::GameView]);
    let [_viewport, side_bar] = tree.split_right(
        egui_dock::NodeIndex::root(),
        0.7,
        vec![EguiWindow::ECS(layout_settings_tab)],
    );
    let [_, bottom_region] =
        tree.split_below(side_bar, 1.0 / 3.0, vec![EguiWindow::ECS(cameras_tab)]);
    tree.split_below(bottom_region, 1.0 / 2.0, vec![EguiWindow::ECS(replay_tab)]);
    world.insert_resource(UiState::new(tree));
}

#[derive(Resource, Default)]
struct GroundTexture {
    handle: Handle<Image>,
}

fn set_ground_sampler(
    mut ev_asset: EventReader<AssetEvent<Image>>,
    mut assets: ResMut<Assets<Image>>,
    map_img: Res<GroundTexture>,
    mut q_ground_plane: Query<&mut Handle<StandardMaterial>, With<GroundPlane>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
) {
    for ev in ev_asset.iter() {
        dbg!(&ev);

        // match ev {
        //     AssetEvent::Created { handle } => {
        //         if let Some(texture) = assets.get_mut(&handle) {
        //             texture.sampler_descriptor = ImageSampler::nearest();
        //         }
        //         if let Some(texture) = assets.get_mut(&handle) {
        //             dbg!(&texture.sampler_descriptor);
        //         }
        //     }
        //     _ => {}
        // }

        match ev {
            AssetEvent::Created { handle } => {
                // WARNING: this mutable access will cause another
                // AssetEvent (Modified) to be emitted!

                if *handle == map_img.handle {
                    let texture = assets.get_mut(handle).unwrap();
                    texture.sampler_descriptor = ImageSampler::Descriptor(SamplerDescriptor {
                        mag_filter: FilterMode::Nearest,
                        min_filter: FilterMode::Linear,
                        address_mode_u: AddressMode::Repeat,
                        address_mode_v: AddressMode::Repeat,
                        address_mode_w: AddressMode::Repeat,
                        ..Default::default()
                    });
                    let ground_material_handle = q_ground_plane.get_single().unwrap();
                    let ground_material =
                        standard_materials.get_mut(ground_material_handle).unwrap();
                    ground_material.base_color_texture = Some(handle.clone());
                }
            }
            AssetEvent::Modified { handle: _ } => {}
            AssetEvent::Removed { handle: _ } => {}
        }
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    mut ambient_light: ResMut<AmbientLight>,
) {
    let ground_texture_handle = asset_server.load("textures/checker.png");
    commands.insert_resource(GroundTexture {
        handle: ground_texture_handle.clone(),
    });

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
        GroundPlane,
        PbrBundle {
            mesh: meshes.add(ground_mesh),

            // shape::Quad::from_size(100f32).into()),
            transform: Transform::from_translation(vec3(0.5, 0.0, 0.5)),
            material: standard_materials.add(StandardMaterial {
                // base_color: Color::rgb(0.3, 0.5, 0.3),`
                base_color_texture: Some(ground_texture_handle),
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
            label: Some("Layout".to_string()),
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
                    PassDebugSettings {
                        blocks: true,
                        arcs: true,
                    },
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
struct ReplayPassProgress {
    remainder: f32,
    current: usize,
    length: usize,
    duration: f32,
    playing: bool,
}
impl Default for ReplayPassProgress {
    fn default() -> Self {
        Self {
            remainder: 0.0,
            current: 0,
            length: 0,
            duration: 2.5,
            playing: false,
        }
    }
}

#[derive(Component)]
struct ReplayOrder(usize);

fn replay_generation_system(
    mut commands: Commands,
    mut q_passes: Query<(
        &mut ReplayPassProgress,
        &WfcFCollapsedData,
        Option<&ReplayTileMapMaterials>,
        &Children,
    )>,
    q_blocks: Query<&mut DebugBlocks>,
    q_tiles: Query<(Entity, &ReplayOrder)>,
    time: Res<Time>,
    mut tile_materials: ResMut<Assets<TilePbrMaterial>>,
) {
    for (mut progress, _collapsed_data, materials, children) in q_passes.iter_mut() {
        for DebugBlocks { material_handle } in q_blocks.iter_many(children) {
            if let Some(material) = tile_materials.get_mut(&material_handle) {
                material.order_cut_off = progress.current as u32;
            };
        }

        for (tile_entity, ReplayOrder(tile_order)) in q_tiles.iter_many(children) {
            commands
                .entity(tile_entity)
                .insert(if tile_order >= &progress.current {
                    Visibility::Hidden
                } else {
                    Visibility::Visible
                });
        }

        if let Some(ReplayTileMapMaterials(materials)) = materials {
            for material in materials.iter() {
                tile_materials
                    .get_mut(material)
                    .expect("Entity with non-existent tilemap material")
                    .order_cut_off = progress.current as u32;
            }
        }

        if progress.playing {
            progress.remainder +=
                time.delta_seconds() * (progress.length as f32 / progress.duration);
            progress.current += progress.remainder as usize;
            progress.remainder = progress.remainder.rem_euclid(1.0);

            if progress.current >= progress.length {
                progress.current = progress.length;
                progress.playing = false;
                progress.remainder = 0.0;
            }
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
    asset_server: Res<AssetServer>,
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
                    commands
                        .spawn((
                            SceneBundle {
                                scene: asset_server.load(path),
                                transform,
                                ..Default::default()
                            },
                            ReplayOrder(collapsed_data.graph.order[node_index]),
                        ))
                        .set_parent(entity);
                }
            }
        }

        commands.entity(entity).remove::<GenerateMeshMarker>();
    }
}

#[derive(Component)]
struct ReplayTileMapMaterials(Vec<Handle<TilePbrMaterial>>);

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
    mut tile_materials: ResMut<Assets<TilePbrMaterial>>,
) {
    let fira_code_handle = asset_server.load("fonts/FiraCode-Bold.ttf");

    for (entity, facade_pass_data, collapsed_data, tileset, debug_settings) in query.iter_mut() {
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
                    404 => error_mesh_builder.add_mesh(
                        &error_cube,
                        transform,
                        collapsed_data.graph.order[index] as u32,
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
                                            color: Color::rgb(0.9, 0.4, 0.4),
                                        },
                                    }])
                                    .with_alignment(TextAlignment::Center),
                                    ..default()
                                },
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
                        collapsed_data.graph.order[index + facade_pass_data.vertices.len()] as u32,
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
                            collapsed_data.graph.order[index + facade_pass_data.vertices.len()]
                                as u32,
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
                        collapsed_data.graph.order
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
                            collapsed_data.graph.order[index
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
            commands.entity(entity).insert(ReplayTileMapMaterials {
                0: vec![
                    error_material,
                    vertex_material,
                    edge_material,
                    quad_material,
                ],
            });
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
            .insert(ReplayPassProgress {
                length: collapsed_data.graph.order.len(),
                current: collapsed_data.graph.order.len(),
                ..Default::default()
            })
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
            commands
                .entity(entity)
                .insert(ReplayTileMapMaterials { 0: vec![material] });
        }
        if debug_settings.arcs {}
    }
}

struct EcsUiLayout {
    system_state: SystemState<(
        Commands<'static, 'static>,
        Query<'static, 'static, Entity, With<WfcEntityMarker>>,
        Local<'static, LayoutGraphSettings>,
        Query<'static, 'static, &'static mut Handle<Mesh>, With<GroundPlane>>,
        ResMut<'static, Assets<Mesh>>,
        Gizmos<'static>,
        ResMut<'static, GizmoConfig>,
    )>,
}

impl EcsUiLayout {
    fn new(world: &mut World) -> Box<dyn EcsUiNode + Send + Sync + 'static> {
        Box::new(Self {
            system_state: SystemState::new(world),
        })
    }
}

impl std::fmt::Debug for EcsUiLayout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EcsUiLayout").finish()
    }
}

impl EcsUiNode for EcsUiLayout {
    fn ui(
        &mut self,
        world: &mut World,
        ui: &mut egui::Ui,
        _type_registry: &bevy_reflect::TypeRegistry,
    ) {
        let (
            mut commands,
            wfc_entities,
            mut layout_settings,
            mut q_ground,
            mut meshes,
            mut gizmos,
            mut config,
        ) = self.system_state.get_mut(world);

        {
            config.line_width = 2.0;
            let origin = vec3(-2.0, 0.0, -2.0);
            let max = vec3(2.0, 3.0, 2.0)
                * vec3(
                    layout_settings.x_size as f32 + 2.0,
                    layout_settings.y_size as f32 + 1.0,
                    layout_settings.z_size as f32 + 2.0,
                );
            let e_x = max * Vec3::X;
            let e_y = max * Vec3::Y;
            let e_z = max * Vec3::Z;

            let bound_color = Color::rgb(0.95, 0.95, 0.95);

            gizmos.line(origin, origin + e_x, Color::rgb(0.9, 0.2, 0.2));
            gizmos.line(origin, origin + e_y, Color::rgb(0.2, 0.9, 0.2));
            gizmos.line(origin, origin + e_z, Color::rgb(0.2, 0.2, 0.9));

            gizmos.line(origin + e_x, origin + e_x + e_y, bound_color);
            gizmos.line(origin + e_x, origin + e_x + e_z, bound_color);

            gizmos.line(origin + e_y, origin + e_y + e_z, bound_color);
            gizmos.line(origin + e_y, origin + e_y + e_x, bound_color);

            gizmos.line(origin + e_z, origin + e_z + e_x, bound_color);
            gizmos.line(origin + e_z, origin + e_z + e_y, bound_color);

            gizmos.line(origin + e_x + e_y + e_z, origin + e_x + e_y, bound_color);
            gizmos.line(origin + e_x + e_y + e_z, origin + e_y + e_z, bound_color);
            gizmos.line(origin + e_x + e_y + e_z, origin + e_z + e_x, bound_color);
        }

        ui.label("Layout size");
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("x:")
                    .monospace()
                    .color(egui::Rgba::from_rgb(0.8, 0.2, 0.2)),
            );
            ui.add(egui::DragValue::new(&mut layout_settings.x_size));
            ui.label(
                egui::RichText::new("y:")
                    .monospace()
                    .color(egui::Rgba::from_rgb(0.2, 0.8, 0.2)),
            );
            ui.add(egui::DragValue::new(&mut layout_settings.y_size));
            ui.label(
                egui::RichText::new("z:")
                    .monospace()
                    .color(egui::Rgba::from_rgb(0.2, 0.2, 0.8)),
            );
            ui.add(egui::DragValue::new(&mut layout_settings.z_size));
        });

        ui.add_space(12.0);
        if ui.button("Generate").clicked() {
            for entity in wfc_entities.iter() {
                commands.entity(entity).insert(WfcInvalidatedMarker);
            }

            let layout_entity = commands
                .spawn((
                    WfcEntityMarker,
                    WfcPassReadyMarker,
                    GenerateDebugMarker,
                    LayoutPass {
                        settings: *layout_settings,
                    },
                    PassDebugSettings {
                        blocks: true,
                        arcs: true,
                    },
                ))
                .id();

            commands.spawn((
                WfcEntityMarker,
                FacadePassSettings,
                WfcPendingParentMarker,
                WfcParentPasses(vec![layout_entity]),
            ));

            if let Ok(ground) = q_ground.get_single_mut() {
                let padding = vec3(10.0, 0.0, 10.0);
                let start = vec3(-1.5, 0.0, -1.5) - padding;
                let end = vec3(
                    2.0 * layout_settings.x_size as f32,
                    0.0,
                    2.0 * layout_settings.z_size as f32,
                ) + vec3(0.5, 0.0, 0.5)
                    + padding;

                let mut ground_mesh =
                    Mesh::new(bevy::render::render_resource::PrimitiveTopology::TriangleStrip);
                ground_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0., 1., 0.]; 4]);
                ground_mesh.insert_attribute(
                    Mesh::ATTRIBUTE_UV_0,
                    vec![
                        [0.5 * start.x, 0.5 * start.z],
                        [0.5 * start.x, 0.5 * end.z],
                        [0.5 * end.x, 0.5 * start.z],
                        [0.5 * end.x, 0.5 * end.z],
                    ],
                );
                ground_mesh.insert_attribute(
                    Mesh::ATTRIBUTE_POSITION,
                    vec![
                        vec3(start.x, 0.0, start.z),
                        vec3(start.x, 0.0, end.z),
                        vec3(end.x, 0.0, start.z),
                        vec3(end.x, 0.0, end.z),
                    ],
                );
                let _ = meshes.set(ground.id(), ground_mesh);
            }
        }
        if ui.button("Reset").clicked() {
            for entity in wfc_entities.iter() {
                commands.entity(entity).insert(WfcInvalidatedMarker);
            }
        }
        self.system_state.apply(world);
    }
}

struct EcsUiCameras {
    system_state: SystemState<
        Query<
            'static,
            'static,
            (
                &'static mut SwitchingCameraController,
                &'static mut Projection,
                Option<&'static mut FpsCameraSettings>,
            ),
        >,
    >,
}

impl EcsUiCameras {
    fn new(world: &mut World) -> Box<dyn EcsUiNode + Send + Sync + 'static> {
        Box::new(Self {
            system_state: SystemState::new(world),
        })
    }
}

impl std::fmt::Debug for EcsUiCameras {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EcsUiCameras").finish()
    }
}

impl EcsUiNode for EcsUiCameras {
    fn ui(
        &mut self,
        world: &mut World,
        ui: &mut egui::Ui,
        type_registry: &bevy_reflect::TypeRegistry,
    ) {
        let mut q_cameras = self.system_state.get_mut(world);

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
                        reflect_inspector::ui_for_value(settings.as_mut(), ui, type_registry);
                    }
                }
                CameraController::PanOrbit => {}
            }

            reflect_inspector::ui_for_value(projection.into_inner(), ui, type_registry);
        }

        self.system_state.apply(world);
    }
}

struct EcsUiReplay {
    system_state: SystemState<
        Query<
            'static,
            'static,
            (
                &'static mut ReplayPassProgress,
                &'static WfcFCollapsedData,
                Option<&'static Children>,
            ),
        >,
    >,
}

impl EcsUiReplay {
    fn new(world: &mut World) -> Box<dyn EcsUiNode + Send + Sync + 'static> {
        Box::new(Self {
            system_state: SystemState::new(world),
        })
    }
}

impl std::fmt::Debug for EcsUiReplay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EcsUiCameras").finish()
    }
}

impl EcsUiNode for EcsUiReplay {
    fn ui(
        &mut self,
        world: &mut World,
        ui: &mut egui::Ui,
        _type_registry: &bevy_reflect::TypeRegistry,
    ) {
        let mut q_passes = self.system_state.get_mut(world);

        ui.horizontal(|ui| {
            if ui.button("Hide all").clicked() {
                for (_, (mut replay_pass, _data, _children)) in q_passes.iter_mut().enumerate() {
                    replay_pass.current = 0;
                }
            }
            if ui.button("Show all").clicked() {
                for (_, (mut replay_pass, _data, _children)) in q_passes.iter_mut().enumerate() {
                    replay_pass.current = replay_pass.length;
                }
            }
        });
        for (_, (mut replay_pass, data, _children)) in q_passes.iter_mut().enumerate() {
            ui.collapsing(
                format!("{}", data.label.as_deref().unwrap_or("Unnamed Pass")),
                |ui| {
                    ui.horizontal(|ui| {
                        if replay_pass.playing {
                            if ui.button("Pause").clicked() {
                                replay_pass.playing = false;
                            }
                        } else {
                            if ui.button("Play").clicked() {
                                replay_pass.playing = true;
                                if replay_pass.current >= replay_pass.length {
                                    replay_pass.current = 0;
                                }
                            }
                        }
                        if replay_pass.current == 0 {
                            if ui.button("Show").clicked() {
                                replay_pass.current = replay_pass.length;
                            }
                        } else {
                            if ui.button("Hide").clicked() {
                                replay_pass.current = 0;
                            }
                        }
                    });
                    let progress = (replay_pass.current as f32
                        / (replay_pass.length as f32).max(1.0))
                    .clamp(0.0, 1.0);

                    let mut updated_progress = progress.clone();
                    ui.add(egui::Slider::new(&mut updated_progress, 0f32..=1f32).show_value(false));
                    if progress != updated_progress {
                        replay_pass.current = ((replay_pass.length as f32).max(0.0)
                            * updated_progress.clamp(0.0, 1.0))
                            as usize;
                    }
                    ui.horizontal(|ui| {
                        ui.label("Duration:");
                        ui.add(
                            egui::DragValue::new(&mut replay_pass.duration)
                                .suffix("s")
                                .speed(0.1),
                        );
                    });
                },
            );
        }

        self.system_state.apply(world);
    }
}
