use bevy::{asset::ChangeWatcher, tasks::TaskPool, utils::petgraph::Graph};
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
    materials::{debug_arc_material::DebugLineMaterial, tile_pbr_material::TilePbrMaterial},
    tools::MeshBuilder,
    village::{layout_graph::LayoutGraphSettings, layout_pass::LayoutTileset},
    wfc::{Neighbour, TileSet, WaveFunctionCollapse, WfcGraph},
};
use rand::{rngs::StdRng, SeedableRng};

use bevy::tasks::{AsyncComputeTaskPool, Task};

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
    // .set(WindowPlugin {
    //     primary_window: Some(Window {
    //         present_mode: PresentMode::Immediate,
    //         ..Default::default()
    //     }),
    //     ..Default::default()
    // }),))
    .add_plugins(bevy_egui::EguiPlugin)
    .add_systems(Update, ui_system)
    .add_systems(Startup, setup)
    .init_resource::<VillageLoadProgress>()
    .insert_resource(WfcPassPool {
        pool: AsyncComputeTaskPool::init(|| TaskPool::new()),
    })
    .add_event::<VillageWfcEvent>()
    .add_systems(Update, (load_village_system, wfc_passes_system));
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

    commands.insert_resource(GraphSettings::LayoutGraphSettings(
        LayoutGraphSettings::default(),
    ));
}

#[derive(Resource)]
struct VillageResult {
    graph: Arc<WfcGraph<usize>>,
}

#[derive(Component)]
struct VillageTile;

struct VillageWaveFunctionCollapse;
impl VillageWaveFunctionCollapse {
    fn wfc(settings: LayoutGraphSettings) -> Option<Arc<WfcGraph<usize>>> {
        let tileset = LayoutTileset::default();
        let mut graph = tileset.create_graph(&settings);
        let constraints = tileset.get_constraints();
        let mut rng = StdRng::from_entropy();

        WaveFunctionCollapse::collapse(&mut graph, &constraints, &tileset.get_weights(), &mut rng);
        match graph.validate() {
            Ok(result) => Some(Arc::new(result)),
            Err(e) => None,
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

    fn create_debug_arcs(result: Arc<WfcGraph<usize>>, settings: LayoutGraphSettings) -> Mesh {
        let mut arc_vertex_positions = Vec::new();
        let mut arc_vertex_normals = Vec::new();
        let mut arc_vertex_uvs = Vec::new();
        let mut arc_vertex_colors = Vec::new();

        for (u, neighbours) in result.neighbors.iter().enumerate() {
            for Neighbour { index: v, arc_type } in neighbours {
                let color = Self::ARC_COLORS[*arc_type.min(&6)];

                let u = settings.posf32_from_index(u);
                let v = settings.posf32_from_index(*v);
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

        let mut edges = Mesh::new(bevy::render::render_resource::PrimitiveTopology::TriangleList);
        edges.insert_attribute(Mesh::ATTRIBUTE_POSITION, arc_vertex_positions);
        edges.insert_attribute(Mesh::ATTRIBUTE_NORMAL, arc_vertex_normals);
        edges.insert_attribute(Mesh::ATTRIBUTE_UV_0, arc_vertex_uvs);
        edges.insert_attribute(Mesh::ATTRIBUTE_COLOR, arc_vertex_colors);
        return edges;
    }

    fn spawn_arcs(
        commands: &mut Commands,
        meshes: &mut ResMut<Assets<Mesh>>,
        line_materials: &mut ResMut<Assets<DebugLineMaterial>>,
        arcs_mesh: Mesh,
    ) {
        commands.spawn((
            MaterialMeshBundle {
                mesh: meshes.add(arcs_mesh),
                material: line_materials.add(DebugLineMaterial {
                    color: Color::rgb(1.0, 0.0, 1.0),
                }),
                visibility: Visibility::Visible,
                ..Default::default()
            },
            DebugArcs,
        ));
    }

    // fn create_resource_handles() {
    //     let full_box: Mesh = shape::Box::new(1.9, 2.9, 1.9).into();
    //     let node_box: Mesh = shape::Cube::new(0.2).into();
    //     let error_box: Mesh = shape::Cube::new(1.0).into();

    //     let corner_material = tile_materials.add(TilePbrMaterial {
    //         base_color: Color::rgb(0.8, 0.6, 0.6),
    //         ..Default::default()
    //     });

    //     let side_material = tile_materials.add(TilePbrMaterial {
    //         base_color: Color::rgb(0.6, 0.8, 0.6),
    //         ..Default::default()
    //     });

    //     let center_material = tile_materials.add(TilePbrMaterial {
    //         base_color: Color::rgb(0.6, 0.6, 0.8),
    //         ..Default::default()
    //     });

    //     let space_material = tile_materials.add(TilePbrMaterial {
    //         base_color: Color::rgb(0.8, 0.2, 0.2),
    //         ..Default::default()
    //     });

    //     let air_material = tile_materials.add(TilePbrMaterial {
    //         base_color: Color::rgb(0.2, 0.2, 0.8),
    //         ..Default::default()
    //     });

    //     let error_material = tile_materials.add(TilePbrMaterial {
    //         base_color: Color::rgb(1.0, 0.0, 0.0),
    //         ..Default::default()
    //     });

    //     let missing_material = tile_materials.add(TilePbrMaterial {
    //         base_color: Color::rgb(1.0, 0.5, 1.0),
    //         ..Default::default()
    //     });
    // }

    fn create_debug_mesh(
        result: Arc<WfcGraph<usize>>,
        settings: LayoutGraphSettings,
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

    fn spawn(
        commands: &mut Commands,
        meshes: &mut ResMut<Assets<Mesh>>,
        tile_materials: &mut ResMut<Assets<TilePbrMaterial>>,
        line_materials: &mut ResMut<Assets<DebugLineMaterial>>,
        result: Arc<WfcGraph<usize>>,
        mesh_data: (Mesh, Mesh, Option<Collider>),
    ) {
        let material = tile_materials.add(TilePbrMaterial {
            base_color: Color::rgb(0.6, 0.6, 0.8),
            ..Default::default()
        });

        let mut entity_commands = commands.spawn((
            VillageTile,
            MaterialMeshBundle {
                material: material.clone(),
                mesh: meshes.add(mesh_data.0),
                visibility: Visibility::Visible,
                ..Default::default()
            },
        ));
        if let Some(collider) = mesh_data.2 {
            entity_commands.insert((RigidBody::Fixed, collider));
        }

        let mut entity_commands = commands.spawn((
            VillageTile,
            MaterialMeshBundle {
                material: material.clone(),
                mesh: meshes.add(mesh_data.1),
                visibility: Visibility::Visible,
                ..Default::default()
            },
        ));

        commands.insert_resource(VillageResult { graph: result });
    }
}

#[derive(Component)]
struct DebugArcs;

#[derive(Event)]
enum VillageWfcEvent {
    Start,
}

#[derive(Resource)]
struct VillageLoadProgress {
    progress: f32,
    duration: f32,
    playing: bool,
    current: usize,
}
impl Default for VillageLoadProgress {
    fn default() -> Self {
        Self {
            progress: 1.0,
            duration: 2.0,
            playing: false,
            current: 0,
        }
    }
}

fn load_village_system(
    _tiles: Query<&mut Visibility, With<VillageTile>>,
    result: Option<Res<VillageResult>>,
    time: Res<Time>,
    mut tile_materials: ResMut<Assets<TilePbrMaterial>>,
    mut progress: ResMut<VillageLoadProgress>,
) {
    if let Some(result) = result {
        progress.current = (result.graph.order.len() as f32 * progress.progress) as usize;

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

fn ui_system(
    type_registry: ResMut<AppTypeRegistry>,
    mut contexts: bevy_egui::EguiContexts,
    mut progress: ResMut<VillageLoadProgress>,
    mut debug_arcs: Query<&mut Visibility, With<DebugArcs>>,
    mut settings_resource: ResMut<GraphSettings>,
    mut commands: Commands,
    meshes: ResMut<Assets<Mesh>>,
    line_materials: ResMut<Assets<DebugLineMaterial>>,
    mut existing_tiles: Query<Entity, With<VillageTile>>,
    mut existing_debug_arcs: Query<Entity, With<DebugArcs>>,
    tile_materials: ResMut<Assets<TilePbrMaterial>>,
    mut q_cameras: Query<(
        &mut SwitchingCameraController,
        &mut Projection,
        Option<&mut FpsCameraSettings>,
    )>,
    mut ev_village_wfc: EventWriter<VillageWfcEvent>,
) {
    egui::Window::new("Settings and Controls").show(contexts.ctx_mut(), |ui| {
        let settings = settings_resource.as_mut();
        ui.collapsing("WFC Settings", |ui| match settings {
            GraphSettings::LayoutGraphSettings(settings) => {
                ui.heading("Settings for layout graph");
                ui.add(egui::DragValue::new(&mut settings.x_size));
                ui.add(egui::DragValue::new(&mut settings.y_size));
                ui.add(egui::DragValue::new(&mut settings.z_size));
                if ui.button("Generate").clicked() {
                    for tile in existing_tiles.iter_mut() {
                        commands.entity(tile).despawn();
                    }
                    for arcs in existing_debug_arcs.iter_mut() {
                        commands.entity(arcs).despawn();
                    }
                    ev_village_wfc.send(VillageWfcEvent::Start);
                }
            }
        });

        ui.collapsing("Replay", |ui| {
            if progress.playing {
                if ui.button("Pause").clicked() {
                    progress.playing = false;
                }
            } else {
                if ui.button("Play").clicked() {
                    progress.playing = true;
                    if progress.progress >= 1.0 {
                        progress.progress = 0.0;
                    }
                }
            }
            ui.label("Progress");
            ui.add(egui::Slider::new(&mut progress.progress, 0f32..=1f32).show_value(false));
            ui.label("Duration");
            ui.add(egui::DragValue::new(&mut progress.duration).clamp_range(0f32..=20f32));
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

        ui.collapsing("Constraint Arcs", |ui| {
            for (index, mut arc) in debug_arcs.iter_mut().enumerate() {
                let mut show = *arc == Visibility::Visible;
                ui.checkbox(&mut show, format!("Arc set #{}", index));
                if show != (*arc == Visibility::Visible) {
                    *arc = match show {
                        true => Visibility::Visible,
                        false => Visibility::Hidden,
                    };
                }
            }
        });
    });
}

#[derive(Resource)]
enum GraphSettings {
    LayoutGraphSettings(LayoutGraphSettings),
}

#[derive(Resource)]
struct WfcPassPool {
    pool: &'static AsyncComputeTaskPool,
}

#[derive(Component)]
struct WfcPass {
    generate: Box<dyn Fn() -> () + 'static + Sync + Send>,
}

#[derive(Default)]
struct PassState {
    settings: LayoutGraphSettings,
    status: PassStateEnum,
    wfc_result: Option<Arc<WfcGraph<usize>>>,
    wfc_task: Option<Task<Option<Arc<WfcGraph<usize>>>>>,
    debug_arcs_task: Option<Task<Mesh>>,
    debug_mesh_task: Option<Task<(Mesh, Mesh, Option<Collider>)>>,
}
impl PassState {
    fn reset(&mut self) {
        if let Some(task) = self.wfc_task.take() {
            future::block_on(future::poll_once(task.cancel()));
        }
        if let Some(task) = self.debug_arcs_task.take() {
            future::block_on(future::poll_once(task.cancel()));
        }
        if let Some(task) = self.debug_mesh_task.take() {
            future::block_on(future::poll_once(task.cancel()));
        }
        self.wfc_result = None;
    }
}

#[derive(Default)]
enum PassStateEnum {
    #[default]
    Idle,
    Start,
    WfcPending,
    DebugMeshesPending((bool, bool)),
}

fn wfc_passes_system(
    task_pool_resource: Res<WfcPassPool>,
    mut state: Local<PassState>,

    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut line_materials: ResMut<'_, Assets<DebugLineMaterial>>,
    mut tile_materials: ResMut<Assets<TilePbrMaterial>>,
    mut ev_village_wfc: EventReader<VillageWfcEvent>,
    graph_settings: Res<GraphSettings>,
) {
    match graph_settings.as_ref() {
        GraphSettings::LayoutGraphSettings(settings) => {
            state.settings = *settings;
        }
    }

    for event in ev_village_wfc.iter() {
        match event {
            VillageWfcEvent::Start => {
                state.status = PassStateEnum::Start;
            }
        }
    }

    match state.status {
        PassStateEnum::Start => {
            state.reset();
            let settings = state.settings;
            state.wfc_task = Some(
                task_pool_resource
                    .pool
                    .spawn(async move { VillageWaveFunctionCollapse::wfc(settings) }),
            );
            state.status = PassStateEnum::WfcPending;
        }
        PassStateEnum::WfcPending => {
            if let Some(mut task) = state.wfc_task.as_mut() {
                if task.is_finished() {
                    dbg!("WFC task finished");
                    if let Some(Some(result_arc)) = future::block_on(future::poll_once(&mut task)) {
                        let settings = state.settings;
                        let result_arc_a = result_arc.clone();
                        let result_arc_b = result_arc.clone();
                        state.wfc_result = Some(result_arc);
                        state.debug_arcs_task = Some(task_pool_resource.pool.spawn(async move {
                            VillageWaveFunctionCollapse::create_debug_arcs(result_arc_a, settings)
                        }));
                        state.debug_mesh_task = Some(task_pool_resource.pool.spawn(async move {
                            VillageWaveFunctionCollapse::create_debug_mesh(result_arc_b, settings)
                        }));
                        state.status = PassStateEnum::DebugMeshesPending((false, false));
                    }
                }
            }
        }
        PassStateEnum::DebugMeshesPending((mut arcs_generated, mut mesh_generated)) => {
            if !arcs_generated {
                if let Some(mut task) = state.debug_arcs_task.as_mut() {
                    if task.is_finished() {
                        arcs_generated = true;
                        dbg!("Arc task finished");

                        if let Some(debug_arcs) =
                            future::block_on(future::poll_once(&mut task)).clone()
                        {
                            VillageWaveFunctionCollapse::spawn_arcs(
                                &mut commands,
                                &mut meshes,
                                &mut line_materials,
                                debug_arcs,
                            )
                        }
                    }
                }
            }
            if !mesh_generated {
                if let Some(mut task) = state.debug_mesh_task.as_mut() {
                    if task.is_finished() {
                        mesh_generated = true;
                        dbg!("Mesh task finished");

                        if let Some(debug_meshes) =
                            future::block_on(future::poll_once(&mut task)).clone()
                        {
                            if let Some(result) = state.wfc_result.take() {
                                VillageWaveFunctionCollapse::spawn(
                                    &mut commands,
                                    &mut meshes,
                                    &mut tile_materials,
                                    &mut line_materials,
                                    result,
                                    debug_meshes,
                                );
                            }
                        }
                    }
                }
            }
            if arcs_generated && mesh_generated {
                state.status = PassStateEnum::Idle;
            } else {
                state.status = PassStateEnum::DebugMeshesPending((arcs_generated, mesh_generated))
            }
        }
        PassStateEnum::Idle => {}
    };
}
