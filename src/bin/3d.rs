use std::f32::consts::PI;
use std::time::Duration;

use bevy::asset::ChangeWatcher;

use bevy::math::{vec2, vec3, vec4};
use bevy::prelude::shape::Cube;
use bevy::prelude::*;
use bevy::prelude::{AssetPlugin, PluginGroup};
use bevy::render::mesh::{MeshVertexAttribute, MeshVertexAttributeId};
use bevy::render::primitives::Sphere;
use bevy::render::render_resource::{AddressMode, FilterMode, SamplerDescriptor, VertexFormat};
use bevy::time::Stopwatch;
use bevy_inspector_egui::{bevy_egui, egui};
use bevy_mod_debugdump;
use hierarchical_wfc::castle_tilset::CastleTileset;
use hierarchical_wfc::debug_line::DebugLineMaterial;
use hierarchical_wfc::graph::{Graph, Neighbor};
use hierarchical_wfc::graph_grid::GridGraphSettings;
use hierarchical_wfc::pan_orbit_cam::PanOrbitCameraPlugin;
use hierarchical_wfc::tileset::TileSet;
use hierarchical_wfc::village::layout_graph::{self, LayoutGraphSettings};
use hierarchical_wfc::village::layout_pass::LayoutTileset;
use hierarchical_wfc::wfc::GraphWfc;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

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
        PanOrbitCameraPlugin,
    ))
    .add_plugins(MaterialPlugin::<DebugLineMaterial>::default())
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
    .add_systems(Update, load_village_system);
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
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut line_materials: ResMut<Assets<DebugLineMaterial>>,
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
    commands.spawn(PbrBundle {
        mesh: meshes.add(ground_mesh),

        // shape::Quad::from_size(100f32).into()),
        transform: Transform::from_translation(vec3(0.5, 0.0, 0.5)),
        material: materials.add(StandardMaterial {
            // base_color: Color::rgb(0.3, 0.5, 0.3),`
            base_color_texture: Some(ground_texture),
            perceptual_roughness: 1.0,
            ..Default::default()
        }),
        ..default()
    });

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

    // create_castle(commands, asset_server);
    create_village(commands, asset_server, meshes, materials, line_materials);
}

fn create_castle(mut commands: Commands, asset_server: Res<AssetServer>) {
    let wall_full_scene: Handle<Scene> = asset_server.load("gltf/castle/w-short-full.gltf#Scene0");
    let wall_slit_scene: Handle<Scene> = asset_server.load("gltf/castle/w-short-slit.gltf#Scene0");
    let wall_window_scene: Handle<Scene> =
        asset_server.load("gltf/castle/w-short-window.gltf#Scene0");

    let pillar_scene: Handle<Scene> = asset_server.load("gltf/castle/p-short.gltf#Scene0");
    let mut rng = rand::thread_rng();

    let settings = GridGraphSettings {
        width: 32,
        height: 32,
        ..Default::default()
    };
    let tileset = CastleTileset::default();

    let create_graph_span = info_span!("wfc_create_graph").entered();
    let mut graph = tileset.create_graph(&settings);
    create_graph_span.exit();

    let setup_constraints_span = info_span!("wfc_setup_constraints").entered();
    let constraints = tileset.get_constraints();
    let mut rng = StdRng::from_entropy();
    setup_constraints_span.exit();

    let collapse_span = info_span!("wfc_collapse").entered();
    GraphWfc::collapse(&mut graph, &constraints, &tileset.get_weights(), &mut rng);
    collapse_span.exit();

    let render_span = info_span!("wfc_render").entered();
    let result = match graph.validate() {
        Ok(graph) => graph,
        Err(e) => {
            println!("Failed to generate!");
            println!("{}", e);
            return;
        }
    };

    // result
    for i in 0..result.tiles.len() {
        let mut tile_index = result.tiles[i] as usize;

        let pos = IVec3::new(
            (i / settings.height) as i32,
            0i32,
            (i % settings.height) as i32,
        );
        let pos = 4.0
            * vec3(
                pos.x.div_euclid(2) as f32,
                pos.y as f32,
                pos.z.div_euclid(2) as f32,
            );

        match tile_index {
            0 => {
                let pos = pos + Vec3::X * 2.0;
                commands.spawn(SceneBundle {
                    scene: wall_full_scene.clone(),
                    transform: Transform::from_translation(pos),
                    ..default()
                });
            }
            1 => {
                let pos = pos + Vec3::Z * 2.0;
                commands.spawn(SceneBundle {
                    scene: wall_full_scene.clone(),
                    transform: Transform::from_translation(pos)
                        * Transform::from_rotation(Quat::from_rotation_y(0.5 * PI)),
                    ..default()
                });
            }
            2 => {
                let pos = pos + Vec3::X * 2.0;
                commands.spawn(SceneBundle {
                    scene: wall_slit_scene.clone(),
                    transform: Transform::from_translation(pos),
                    ..default()
                });
            }
            3 => {
                let pos = pos + Vec3::Z * 2.0;
                commands.spawn(SceneBundle {
                    scene: wall_slit_scene.clone(),
                    transform: Transform::from_translation(pos)
                        * Transform::from_rotation(Quat::from_rotation_y(0.5 * PI)),
                    ..default()
                });
            }
            4 => {
                let pos = pos + Vec3::X * 2.0;
                commands.spawn(SceneBundle {
                    scene: wall_window_scene.clone(),
                    transform: Transform::from_translation(pos),
                    ..default()
                });
            }
            5 => {
                let pos = pos + Vec3::Z * 2.0;
                commands.spawn(SceneBundle {
                    scene: wall_window_scene.clone(),
                    transform: Transform::from_translation(pos)
                        * Transform::from_rotation(Quat::from_rotation_y(0.5 * PI)),
                    ..default()
                });
            }
            6 => {
                commands.spawn(SceneBundle {
                    scene: pillar_scene.clone(),
                    transform: Transform::from_translation(pos),
                    ..default()
                });
            }
            _ => {}
        }

        dbg!(tile_index);
    }
}

#[derive(Resource)]
struct VillageResult {
    graph: Graph<usize>,
    tiles: Vec<Entity>,
}

#[derive(Component)]
struct VillageTile;

fn create_village(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut line_materials: ResMut<Assets<DebugLineMaterial>>,
) {
    let settings = LayoutGraphSettings {
        periodic: false,
        x_size: 32,
        y_size: 3,
        z_size: 32,
    };

    let tileset = LayoutTileset::default();
    let mut graph = tileset.create_graph(&settings);
    let constraints = tileset.get_constraints();
    let mut rng = StdRng::from_entropy();
    GraphWfc::collapse(&mut graph, &constraints, &tileset.get_weights(), &mut rng);
    let result = match graph.validate() {
        Ok(graph) => graph,
        Err(e) => {
            println!("Failed to generate!");
            println!("{}", e);
            return;
        }
    };

    let arcs_mesh = create_arcs(&result, &settings);

    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(arcs_mesh),
        material: line_materials.add(DebugLineMaterial {
            color: Color::rgb(1.0, 0.0, 1.0),
        }),
        ..Default::default()
    });

    let full_box = meshes.add(shape::Box::new(1.9, 2.9, 1.9).into());
    let node_box = meshes.add(shape::Cube::new(0.2).into());

    let mut tiles: Vec<Entity> = Vec::with_capacity(result.tiles.len());
    for (index, tile) in result.tiles.iter().enumerate() {
        let position = settings.posf32_from_index(index);
        let entity = commands.spawn((
            VillageTile,
            match tile {
                0..=3 => PbrBundle {
                    material: materials.add(StandardMaterial {
                        base_color: Color::rgb(0.8, 0.6, 0.6),
                        ..Default::default()
                    }),
                    mesh: full_box.clone(),
                    transform: Transform::from_translation(position),
                    visibility: Visibility::Hidden,
                    ..Default::default()
                },
                4..=7 => PbrBundle {
                    material: materials.add(StandardMaterial {
                        base_color: Color::rgb(0.6, 0.8, 0.6),
                        ..Default::default()
                    }),
                    mesh: full_box.clone(),
                    transform: Transform::from_translation(position),
                    visibility: Visibility::Hidden,

                    ..Default::default()
                },
                8 => PbrBundle {
                    material: materials.add(StandardMaterial {
                        base_color: Color::rgb(0.6, 0.6, 0.8),
                        ..Default::default()
                    }),
                    mesh: full_box.clone(),
                    transform: Transform::from_translation(position),
                    visibility: Visibility::Hidden,
                    ..Default::default()
                },
                9..=12 => PbrBundle {
                    material: materials.add(StandardMaterial {
                        base_color: Color::rgb(0.8, 0.2, 0.2),
                        ..Default::default()
                    }),
                    mesh: node_box.clone(),
                    transform: Transform::from_translation(position),
                    visibility: Visibility::Hidden,
                    ..Default::default()
                },
                _ => PbrBundle {
                    material: materials.add(StandardMaterial {
                        base_color: Color::rgb(0.2, 0.2, 0.8),
                        ..Default::default()
                    }),
                    mesh: node_box.clone(),

                    transform: Transform::from_translation(position),
                    visibility: Visibility::Hidden,
                    ..Default::default()
                },
            },
        ));

        tiles.push(entity.id());
    }
    commands.insert_resource(VillageResult {
        graph: result,
        tiles,
    });
}

fn create_arcs(
    // commands: Commands,
    // mut meshes: ResMut<Assets<Mesh>>,
    // mut line_materials: ResMut<Assets<DebugLineMaterial>>,
    result: &Graph<usize>,
    settings: &LayoutGraphSettings,
) -> Mesh {
    let mut arc_vertex_positions = Vec::new();
    let mut arc_vertex_normals = Vec::new();
    let mut arc_vertex_uvs = Vec::new();
    let mut arc_vertex_colors = Vec::new();

    for (u, neighbours) in result.neighbors.iter().enumerate() {
        for Neighbor { index: v, arc_type } in neighbours {
            let color = color_arc(*arc_type);

            let u = settings.posf32_from_index(u);
            let v = settings.posf32_from_index(*v);
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

fn color_arc(arc_type: usize) -> Vec4 {
    match arc_type {
        0 => vec4(1.0, 0.1, 0.1, 1.0),
        1 => vec4(0.1, 1.0, 1.0, 1.0),
        2 => vec4(0.1, 1.0, 0.1, 1.0),
        3 => vec4(1.0, 0.1, 1.0, 1.0),
        4 => vec4(0.1, 0.1, 1.0, 1.0),
        5 => vec4(1.0, 1.0, 0.1, 1.0),
        _ => vec4(0.1, 0.1, 0.1, 1.0),
    }
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
            progress: 0.0,
            duration: 2.0,
            playing: false,
            current: 0,
        }
    }
}

fn load_village_system(
    mut tiles: Query<&mut Visibility, With<VillageTile>>,
    result: Res<VillageResult>,
    time: Res<Time>,
    mut progress: ResMut<VillageLoadProgress>,
) {
    let new_index = (result.tiles.len() as f32 * progress.progress) as usize;

    if new_index > progress.current {
        for index in progress.current..new_index {
            if let Some(current) = result.tiles.get(index) {
                if let Ok(mut visibility) = tiles.get_mut(*current) {
                    *visibility = Visibility::Visible;
                }
            }
        }
    }
    if new_index < progress.current {
        for index in new_index..progress.current {
            if let Some(current) = result.tiles.get(index) {
                if let Ok(mut visibility) = tiles.get_mut(*current) {
                    *visibility = Visibility::Hidden;
                }
            }
        }
    }

    if progress.playing {
        progress.current = new_index;
        if progress.progress > 1.0 {
            progress.playing = false;
            progress.progress = 1.0;
        }
        progress.progress += time.delta_seconds() / progress.duration;
    }
}

fn ui_system(mut contexts: bevy_egui::EguiContexts, mut progress: ResMut<VillageLoadProgress>) {
    egui::Window::new("Replay Generation").show(contexts.ctx_mut(), |ui| {
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
}
