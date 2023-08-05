use std::f32::consts::PI;
use std::time::Duration;

use bevy::asset::ChangeWatcher;

use bevy::math::{vec2, vec3, vec4};
use bevy::prelude::shape::Cube;
use bevy::prelude::*;
use bevy::prelude::{AssetPlugin, PluginGroup};
use bevy::render::mesh::{
    Indices, MeshVertexAttribute, MeshVertexAttributeId, VertexAttributeValues,
};
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
use hierarchical_wfc::tile_pbr_material::TilePbrMaterial;
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
    commands.spawn(PbrBundle {
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

    commands.insert_resource(GraphSettings::LayoutGraphSettings(
        LayoutGraphSettings::default(),
    ));

    // create_castle(commands, asset_server);
    // create_village(commands, asset_server, meshes, materials, line_materials);
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
    for i in 0..result.nodes.len() {
        let mut tile_index = result.nodes[i] as usize;

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
    }
}

#[derive(Resource)]
struct VillageResult {
    graph: Graph<usize>,
}

#[derive(Component)]
struct VillageTile;

fn create_village(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut tile_materials: ResMut<Assets<TilePbrMaterial>>,
    mut line_materials: ResMut<Assets<DebugLineMaterial>>,
    settings: &LayoutGraphSettings,
) {
    let tileset = LayoutTileset::default();
    let mut graph = tileset.create_graph(settings);
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

    let arcs_mesh = create_arcs(&result, settings);

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

    let full_box: Mesh = shape::Box::new(1.9, 2.9, 1.9).into();
    let node_box: Mesh = shape::Cube::new(0.2).into();
    let error_box: Mesh = shape::Cube::new(1.0).into();

    let mut corner_mesh_builder = MeshBuilder::new();
    let corner_material = tile_materials.add(TilePbrMaterial {
        base_color: Color::rgb(0.8, 0.6, 0.6),
        ..Default::default()
    });

    let mut side_mesh_builder = MeshBuilder::new();
    let side_material = tile_materials.add(TilePbrMaterial {
        base_color: Color::rgb(0.6, 0.8, 0.6),
        ..Default::default()
    });

    let mut center_mesh_builder = MeshBuilder::new();
    let center_material = tile_materials.add(TilePbrMaterial {
        base_color: Color::rgb(0.6, 0.6, 0.8),
        ..Default::default()
    });

    let mut space_mesh_builder = MeshBuilder::new();
    let space_material = tile_materials.add(TilePbrMaterial {
        base_color: Color::rgb(0.8, 0.2, 0.2),
        ..Default::default()
    });

    let mut air_mesh_builder = MeshBuilder::new();
    let air_material = tile_materials.add(TilePbrMaterial {
        base_color: Color::rgb(0.2, 0.2, 0.8),
        ..Default::default()
    });

    let mut error_mesh_builder = MeshBuilder::new();
    let error_material = tile_materials.add(TilePbrMaterial {
        base_color: Color::rgb(1.0, 0.0, 0.0),
        ..Default::default()
    });

    let mut missing_mesh_builder = MeshBuilder::new();
    let missing_material = tile_materials.add(TilePbrMaterial {
        base_color: Color::rgb(1.0, 0.5, 1.0),
        ..Default::default()
    });

    for (index, tile) in result.nodes.iter().enumerate() {
        let position = settings.posf32_from_index(index);
        let transform = Transform::from_translation(position);
        let order = result.order[*tile] as u32;
        match tile {
            0..=3 => corner_mesh_builder.add_mesh(&full_box, transform, order),
            4..=7 => side_mesh_builder.add_mesh(&full_box, transform, order),
            8 => center_mesh_builder.add_mesh(&full_box, transform, order),
            9..=12 => space_mesh_builder.add_mesh(&node_box, transform, order),
            13 => air_mesh_builder.add_mesh(&node_box, transform, order),
            404 => error_mesh_builder.add_mesh(&error_box, transform, order),
            _ => missing_mesh_builder.add_mesh(&error_box, transform, order),
        };
    }
    for (material, mesh_builder) in [
        (corner_material, corner_mesh_builder),
        (side_material, side_mesh_builder),
        (center_material, center_mesh_builder),
        (space_material, space_mesh_builder),
        (air_material, air_mesh_builder),
        (error_material, error_mesh_builder),
        (missing_material, missing_mesh_builder),
    ] {
        let mesh = mesh_builder.build_mesh();
        commands.spawn((
            VillageTile,
            MaterialMeshBundle {
                material: material.clone(),
                mesh: meshes.add(mesh),
                visibility: Visibility::Visible,
                ..Default::default()
            },
        ));
    }

    // if let Some(bundle) = bundle {
    //     let mut entity = commands.spawn(bundle);
    //     entity.insert(VillageTile);
    //     tiles.push(entity.id());
    // }

    commands.insert_resource(VillageResult { graph: result });
}

struct MeshBuilder {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
    order: Vec<u32>,
    offset: u32,
}
impl MeshBuilder {
    fn new() -> Self {
        Self {
            positions: Vec::new(),
            normals: Vec::new(),
            uvs: Vec::new(),
            indices: Vec::new(),
            order: Vec::new(),
            offset: 0,
        }
    }

    fn add_mesh(&mut self, mesh: &Mesh, transform: Transform, order: u32) {
        if let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        {
            self.positions.extend(
                positions
                    .iter()
                    .map(|p| transform * Vec3::from_array(*p))
                    .map(|p| p.to_array()),
            );
            self.offset += positions.len() as u32;
            self.order
                .extend(std::iter::repeat(order).take(positions.len()))
        }

        if let Some(VertexAttributeValues::Float32x3(normals)) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        {
            self.normals.extend(normals);
        }
        if let Some(VertexAttributeValues::Float32x2(uvs)) = mesh.attribute(Mesh::ATTRIBUTE_UV_0) {
            self.uvs.extend(uvs);
        }
        if let Some(Indices::U32(indices)) = mesh.indices() {
            self.indices.extend(indices.iter().map(|i| i + self.offset));
        }
    }
    fn build_mesh(self) -> Mesh {
        const ATTRIBUTE_TILE_ORDER: MeshVertexAttribute =
            MeshVertexAttribute::new("TileOrder", 988540917, VertexFormat::Uint32);

        let mut mesh = Mesh::new(bevy::render::render_resource::PrimitiveTopology::TriangleList);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs);
        mesh.insert_attribute(ATTRIBUTE_TILE_ORDER, self.order);
        mesh.set_indices(Some(Indices::U32(self.indices)));
        mesh
    }
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

#[derive(Component)]
struct DebugArcs;

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
    mut tiles: Query<&mut Visibility, With<VillageTile>>,
    result: Option<Res<VillageResult>>,
    time: Res<Time>,
    mut progress: ResMut<VillageLoadProgress>,
) {
    if let Some(result) = result {
        progress.current = (result.graph.order.len() as f32 * progress.progress) as usize;

        // if new_index > progress.current {
        //     // for index in progress.current..new_index {
        //     //     if let Some(ordered_index) = result.graph.order.get(index) {
        //     //         if let Some(current) = result.tiles.get(*ordered_index) {
        //     //             if let Ok(mut visibility) = tiles.get_mut(*current) {
        //     //                 *visibility = Visibility::Visible;
        //     //             }
        //     //         }
        //     //     }
        //     // }
        // }
        // if new_index < progress.current {
        //     for index in new_index..progress.current {
        //         if let Some(ordered_index) = result.graph.order.get(index) {
        //             if let Some(current) = result.tiles.get(*ordered_index) {
        //                 if let Ok(mut visibility) = tiles.get_mut(*current) {
        //                     *visibility = Visibility::Hidden;
        //                 }
        //             }
        //         }
        //     }
        // }

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
    mut contexts: bevy_egui::EguiContexts,
    mut progress: ResMut<VillageLoadProgress>,
    mut debug_arcs: Query<&mut Visibility, With<DebugArcs>>,
    mut settings_resource: ResMut<GraphSettings>,
    mut commands: Commands,
    meshes: ResMut<Assets<Mesh>>,
    tile_materials: ResMut<Assets<TilePbrMaterial>>,
    line_materials: ResMut<Assets<DebugLineMaterial>>,
    mut existing_tiles: Query<Entity, With<VillageTile>>,
    mut existing_debug_arcs: Query<Entity, With<DebugArcs>>,
) {
    egui::Window::new("WFC Controls").show(contexts.ctx_mut(), |ui| {
        let settings = settings_resource.as_mut();
        ui.collapsing("WFC Graphs", |ui| match settings {
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
                    create_village(commands, meshes, tile_materials, line_materials, settings);
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
        ui.collapsing("Visualisation", |ui| {
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
    });
}

#[derive(Resource)]
enum GraphSettings {
    LayoutGraphSettings(LayoutGraphSettings),
}
