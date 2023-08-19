use bevy::{asset::ChangeWatcher, math::ivec3, render::primitives::Sphere, tasks::TaskPool};
use std::{sync::Arc, time::Duration};

use bevy::{
    math::{vec3, vec4},
    prelude::{AssetPlugin, PluginGroup, *},
    render::render_resource::{AddressMode, FilterMode, SamplerDescriptor},
};
use futures_lite::future;

use bevy_inspector_egui::{bevy_egui, egui, reflect_inspector, DefaultInspectorConfigPlugin};
use bevy_mod_debugdump;
use bevy_rapier3d::prelude::{
    Collider, ComputedColliderShape, NoUserData, RapierPhysicsPlugin, RigidBody,
};
use hierarchical_wfc::{
    camera_controllers::{
        cam_switcher::{CameraController, SwitchingCameraController, SwitchingCameraPlugin},
        fps::FpsCameraSettings,
    },
    materials::{
        debug_arc_material::DebugLineMaterial,
        tile_pbr_material::{self, TilePbrMaterial},
    },
    tools::MeshBuilder,
    village::{
        layout_graph::{create_layout_graph, LayoutGraphSettings},
        layout_pass::LayoutTileset,
    },
    wfc::{Neighbour, Superposition, TileSet, WaveFunctionCollapse, WfcGraph},
};
use itertools;
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
            }),
        SwitchingCameraPlugin,
        RapierPhysicsPlugin::<NoUserData>::default(),
        DefaultInspectorConfigPlugin,
    ))
    .add_plugins(MaterialPlugin::<DebugLineMaterial>::default())
    .add_plugins(MaterialPlugin::<TilePbrMaterial>::default())
    .add_plugins(bevy_egui::EguiPlugin)
    .add_systems(
        Update,
        (
            ui_system,
            wfc_collapse_system,
            wfc_ready_system,
            replay_generation_system,
            layout_init_system,
            layout_debug_system,
            facade_init_system,
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
struct WfcEntityMarker;

#[derive(Component)]
struct LayoutPass {
    settings: LayoutGraphSettings,
}

#[derive(Component)]
struct WfcInitialData {
    graph: WfcGraph<Superposition>,
    constraints: Vec<Vec<Superposition>>,
    weights: Vec<u32>,
    rng: StdRng,
}

#[derive(Component)]
struct WfcFCollapsedData {
    graph: WfcGraph<usize>,
}

#[derive(Component)]
struct WfcParentPasses(Vec<Entity>);

#[derive(Component)]
struct WfcPendingParentMarker;

#[derive(Component)]
struct WfcPassReadyMarker;

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

fn wfc_ready_system(
    mut commands: Commands,
    q_pending: Query<(Entity, &WfcParentPasses), With<WfcPendingParentMarker>>,
    q_parent: Query<With<WfcFCollapsedData>>,
) {
    for (child, WfcParentPasses(parents)) in q_pending.iter() {
        if 'ready: {
            for parent in parents {
                match q_parent.get(*parent) {
                    Ok(_) => {}
                    Err(_) => {
                        break 'ready false;
                    }
                }
            }
            true
        } {
            let mut entity_commands = commands.entity(child);
            entity_commands.remove::<WfcPendingParentMarker>();
            entity_commands.insert(WfcPassReadyMarker);
        }
    }
}

fn wfc_collapse_system(mut commands: Commands, mut query: Query<(Entity, &mut WfcInitialData)>) {
    for (entity, mut initial_data) in query.iter_mut() {
        dbg!("Collapsing Entity");
        let WfcInitialData {
            graph,
            constraints,
            weights,
            rng,
        } = initial_data.as_mut();

        WaveFunctionCollapse::collapse(graph, constraints, weights, rng);
        let mut entity_commands = commands.entity(entity);
        entity_commands.remove::<WfcInitialData>();
        match graph.validate() {
            Ok(result) => {
                entity_commands.insert(WfcFCollapsedData { graph: result });
            }
            Err(_) => {}
        };
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

#[derive(Component)]
struct FacadePass;

const ARC_COLORS: [Vec4; 7] = [
    vec4(1.0, 0.1, 0.1, 1.0), // +x
    vec4(0.1, 1.0, 1.0, 1.0), // -x
    vec4(0.1, 1.0, 0.1, 1.0), // +y
    vec4(1.0, 0.1, 1.0, 1.0), // -y
    vec4(0.1, 0.1, 1.0, 1.0), // +z
    vec4(1.0, 1.0, 0.1, 1.0), // -z
    vec4(0.1, 0.1, 0.1, 1.0), // invalid
];

fn facade_init_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut line_materials: ResMut<Assets<DebugLineMaterial>>,
    query: Query<(Entity, &FacadePass, &WfcParentPasses), With<WfcPassReadyMarker>>,
    q_layout_parents: Query<(&LayoutPass, &WfcFCollapsedData)>,
) {
    for (entity, pass, parents) in query.iter() {
        for (
            LayoutPass {
                settings:
                    LayoutGraphSettings {
                        x_size,
                        y_size,
                        z_size,
                        periodic: _,
                    },
            },
            parent_data,
        ) in q_layout_parents.iter_many(parents.0.iter())
        {
            let mut corner_mesh_builder = MeshBuilder::new();
            let corner_mesh: Mesh = shape::Cube::new(0.5).into();
            let mut nodes: Vec<bool> = vec![false; (x_size + 1) * (y_size + 1) * (z_size + 1)];
            let size = ivec3(*x_size as i32, *y_size as i32, *z_size as i32);
            let node_pos = itertools::iproduct!(0..size.z + 1, 0..size.y + 1, 0..size.x + 1)
                .map(|(z, y, x)| ivec3(x, y, z));
            for (index, pos) in node_pos.clone().enumerate() {
                let mut connected = 0;
                for delta in
                    itertools::iproduct!(-1..=0, -1..=0, -1..=0).map(|(x, y, z)| ivec3(x, y, z))
                {
                    let pos = pos + delta;
                    if (0..size.x).contains(&pos.x)
                        && (0..size.y).contains(&pos.y)
                        && (0..size.z).contains(&pos.z)
                    {
                        let index = pos.dot(ivec3(1, size.x, size.x * size.y)) as usize;

                        let tile = parent_data.graph.nodes[index];
                        if (0..=8).contains(&tile) {
                            connected += 1;
                        }
                    }
                }
                // let index = pos.dot(ivec3(1, size.x + 1, (size.x + 1) * (size.y + 1)));
                dbg!(connected);
                if 0 < connected && connected < 8 {
                    nodes[index as usize] = true;
                    let transform =
                        Transform::from_translation(pos.as_vec3() * vec3(2.0, 3.0, 2.0));
                    corner_mesh_builder.add_mesh(&corner_mesh, transform, 0);
                }
            }

            let mut arc_vertex_positions = Vec::new();
            let mut arc_vertex_normals = Vec::new();
            let mut arc_vertex_uvs = Vec::new();
            let mut arc_vertex_colors = Vec::new();

            for (u, u_pos) in node_pos.enumerate() {
                if !nodes[u] {
                    continue;
                }
                for (arc_type, v_pos) in [
                    IVec3::X,
                    IVec3::NEG_X,
                    IVec3::Y,
                    IVec3::NEG_Y,
                    IVec3::Z,
                    IVec3::NEG_Z,
                ]
                .into_iter()
                .map(|delta| u_pos + delta)
                .enumerate()
                .filter(|(_, pos)| {
                    (0..size.x + 1).contains(&pos.x)
                        && (0..size.y + 1).contains(&pos.y)
                        && (0..size.z + 1).contains(&pos.z)
                })
                .filter(|(_, pos)| {
                    nodes[pos.dot(ivec3(1, size.x + 1, (size.x + 1) * (size.y + 1))) as usize]
                }) {
                    let color = ARC_COLORS[arc_type.min(6)];

                    let u = u_pos.as_vec3() * vec3(2.0, 3.0, 2.0);
                    let v = v_pos.as_vec3() * vec3(2.0, 3.0, 2.0);
                    let normal = (u - v).normalize();

                    arc_vertex_positions.extend([u, v, u, v, v, u]);
                    arc_vertex_normals.extend([
                        Vec3::ZERO,
                        Vec3::ZERO,
                        normal,
                        Vec3::ZERO,
                        normal,
                        normal,
                    ]);

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

            let mut arcs_mesh =
                Mesh::new(bevy::render::render_resource::PrimitiveTopology::TriangleList);
            arcs_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, arc_vertex_positions);
            arcs_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, arc_vertex_normals);
            arcs_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, arc_vertex_uvs);
            arcs_mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, arc_vertex_colors);

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
                    // DebugArcs,
                ))
                .set_parent(entity);

            commands
                .entity(entity)
                .remove::<WfcPassReadyMarker>()
                .insert(MaterialMeshBundle {
                    mesh: meshes.add(corner_mesh_builder.build()),
                    material: materials.add(Color::RED.into()),
                    ..Default::default()
                });
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
            if let Some(material) = tile_materials.get_mut(material_handle) {
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
                base_color: Color::rgb(0.6, 0.6, 0.8),
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
    // mut progress: ResMut<VillageLoadProgress>,
    // mut debug_arcs: Query<&mut Visibility, With<DebugArcs>>,
    // mut settings_resource: ResMut<GraphSettings>,
    mut commands: Commands,
    mut q_passes: Query<(&mut ReplayPassProgress, &WfcFCollapsedData, &Children)>,
    // mut existing_tiles: Query<Entity, With<VillageTile>>,
    // mut existing_debug_arcs: Query<Entity, With<DebugArcs>>,
    wfc_entities: Query<Entity, With<WfcEntityMarker>>,
    mut q_cameras: Query<(
        &mut SwitchingCameraController,
        &mut Projection,
        Option<&mut FpsCameraSettings>,
    )>,
    mut layout_settings: Local<LayoutGraphSettings>,
    // mut ev_village_wfc: EventWriter<VillageWfcEvent>,
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
                            blocks: true,
                            arcs: true,
                        },
                        WfcPassReadyMarker,
                        GenerateDebugMarker,
                    ))
                    .id();

                commands.spawn((
                    WfcEntityMarker,
                    FacadePass,
                    WfcPendingParentMarker,
                    WfcParentPasses(vec![layout_entity]),
                ));
                // for tile in existing_tiles.iter_mut() {
                //     commands.entity(tile).despawn_recursive();
                // }
                // // for arcs in existing_debug_arcs.iter_mut() {
                // //     commands.entity(arcs).despawn_recursive();
                // // }
                // ev_village_wfc.send(VillageWfcEvent::Start);
            }
            if ui.button("Clear").clicked() {
                for entity in wfc_entities.iter() {
                    commands.entity(entity).despawn_recursive();
                }
            }
        });

        ui.collapsing("Replay Generation", |ui| {
            for (index, (mut replay_pass, data, children)) in q_passes.iter_mut().enumerate() {
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

        // ui.collapsing("Constraint Arcs", |ui| {
        //     for (index, mut arc) in debug_arcs.iter_mut().enumerate() {
        //         let mut show = *arc == Visibility::Visible;
        //         ui.checkbox(&mut show, format!("Arc set #{}", index));
        //         if show != (*arc == Visibility::Visible) {
        //             *arc = match show {
        //                 true => Visibility::Visible,
        //                 false => Visibility::Hidden,
        //             };
        //         }
        //     }
        // });
    });
}
