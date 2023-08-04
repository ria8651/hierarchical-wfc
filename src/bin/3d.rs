use std::f32::consts::PI;
use std::time::Duration;

use bevy::asset::ChangeWatcher;

use bevy::math::vec3;
use bevy::pbr::{CascadeShadowConfigBuilder, Cascades};
use bevy::prelude::*;
use bevy::prelude::{AssetPlugin, PluginGroup};

use bevy::render::color;
use bevy::render::render_resource::{AddressMode, FilterMode, SamplerDescriptor};
use rand::{random, Rng};
use wfc_lib::pan_orbit_cam::PanOrbitCameraPlugin;

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
    // .set(WindowPlugin {
    //     primary_window: Some(Window {
    //         present_mode: PresentMode::Immediate,
    //         ..Default::default()
    //     }),
    //     ..Default::default()
    // }),))
    // .add_plugins(bevy_egui::EguiPlugin)
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
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut ambient_light: ResMut<AmbientLight>,
) {
    let wall_full_scene: Handle<Scene> = asset_server.load("gltf/castle/w-short-full.gltf#Scene0");
    let wall_slit_scene: Handle<Scene> = asset_server.load("gltf/castle/w-short-slit.gltf#Scene0");
    let wall_window_scene: Handle<Scene> =
        asset_server.load("gltf/castle/w-short-window.gltf#Scene0");

    let pillar_scene: Handle<Scene> = asset_server.load("gltf/castle/p-short.gltf#Scene0");
    let mut rng = rand::thread_rng();

    for i in 0..5 {
        for j in 0..5 {
            commands.spawn(SceneBundle {
                scene: pillar_scene.clone(),
                transform: Transform::from_translation(vec3(i as f32 * 4.0, 0.0, j as f32 * 4.0)),
                ..default()
            });
            let wall = rng.gen_range(0..4);

            let wall_scene = match wall {
                1 => Some(wall_full_scene.clone()),
                2 => Some(wall_slit_scene.clone()),
                3 => Some(wall_window_scene.clone()),
                _ => None,
            };

            if let Some(wall_scene) = wall_scene {
                if i < 4 {
                    commands.spawn(SceneBundle {
                        scene: wall_scene.clone(),
                        transform: Transform::IDENTITY.with_translation(vec3(
                            i as f32 * 4.0 + 2.0,
                            0.0,
                            j as f32 * 4.0,
                        )),
                        ..default()
                    });
                }
                if j < 4 {
                    commands.spawn(SceneBundle {
                        scene: wall_scene.clone(),
                        transform: Transform::IDENTITY
                            .with_rotation(Quat::from_rotation_y(0.5 * PI))
                            .with_translation(vec3(i as f32 * 4.0, 0.0, j as f32 * 4.0 + 2.0)),
                        ..default()
                    });
                }
            }
            // commands.spawn(SceneBundle {
            //     scene: wall_scene.clone(),
            //     transform: Transform::from_translation(vec3(
            //         i as f32 * 4.0,
            //         0.0,
            //         j as f32 * 4.0 + 2.0,
            //     )),
            //     ..default()
            // });
        }
    }

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

    // commands.spawn(PointLightBundle {
    //     point_light: PointLight {
    //         intensity: 1500.0,
    //         shadows_enabled: true,
    //         ..default()
    //     },
    //     transform: Transform::from_xyz(4.0, 8.0, 4.0),
    //     ..default()
    // });
    // camera
    // commands.spawn(Camera3dBundle {
    //     transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    //     ..default()
    // });

    // commands.spawn((
    //     Camera2dBundle {
    //         camera_render_graph: CameraRenderGraph::new("core_2d"),

    //         projection: OrthographicProjection {
    //             scaling_mode: ScalingMode::AutoMin {
    //                 min_width: 64.0,
    //                 min_height: 64.0,
    //             },
    //             ..Default::default()
    //         },
    //         tonemapping: bevy::core_pipeline::tonemapping::Tonemapping::None,
    //         camera_2d: Camera2d {
    //             clear_color: ClearColorConfig::Custom(Color::hex("2d2a2e").unwrap()),
    //             ..Default::default()
    //         },

    //         transform: Transform::from_translation(Vec3::new(0.5, 0.5, 2.0)),
    //         ..Default::default()
    //     },
    //     MainCamera,
    //     MainPassSettings {},
    // ));
}
