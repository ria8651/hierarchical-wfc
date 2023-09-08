use bevy::{asset::ChangeWatcher, math::Vec4Swizzles, window::PresentMode};
use bevy_egui::EguiContexts;
use bevy_render::texture::ImageSampler;
use constants::{EGUI_X_COLOR, EGUI_Y_COLOR, EGUI_Z_COLOR};
use debug::{
    fragment_debug_destruction_system, fragment_debug_instantiation_system,
    layout_debug_reset_system, layout_debug_visibility_system,
};
use fragments::plugin::GenerationPlugin;

use std::time::Duration;
use tabs::*;

use bevy::{
    math::vec3,
    prelude::{AssetPlugin, PluginGroup, *},
    render::render_resource::{AddressMode, FilterMode, SamplerDescriptor},
};

use bevy::log::LogPlugin;
use bevy_inspector_egui::{bevy_egui, DefaultInspectorConfigPlugin};

use bevy_rapier3d::prelude::{Collider, NoUserData, RapierPhysicsPlugin, RigidBody};
use hierarchical_wfc::{
    camera_plugin::{cam_switcher::SwitchingCameraPlugin, pan_orbit::AlignViewEvent},
    ground_plane_plugin::GroundPlanePlugin,
    materials::{debug_arc_material::DebugLineMaterial, tile_pbr_material::TilePbrMaterial},
    ui_plugin::{EcsUiPlugin, EcsUiState, EcsUiTab},
};

mod debug;
mod fragments;
mod tabs;

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
        GroundPlanePlugin,
        SwitchingCameraPlugin,
        RapierPhysicsPlugin::<NoUserData>::default(),
        DefaultInspectorConfigPlugin,
        MaterialPlugin::<DebugLineMaterial>::default(),
        MaterialPlugin::<TilePbrMaterial>::default(),
        bevy_egui::EguiPlugin,
        EcsUiPlugin,
        GenerationPlugin,
    ))
    .add_systems(
        Update,
        (
            set_ground_sampler,
            fragment_debug_instantiation_system,
            fragment_debug_destruction_system,
            layout_debug_visibility_system,
            layout_debug_reset_system,
            orientation_gizmo_system,
        ),
    )
    .add_systems(Startup, (setup, init_inspector));
    #[cfg(not(target_arch = "wasm32"))]
    {
        let settings = bevy_mod_debugdump::render_graph::Settings::default();
        let dot = bevy_mod_debugdump::render_graph_dot(&app, &settings);
        std::fs::write("render-graph.dot", dot).expect("Failed to write render-graph.dot");
    }
    app.run();
}

#[derive(SystemSet, Hash, Debug, Eq, PartialEq, Clone, Component)]
struct GroundPlane;

fn init_inspector(world: &mut World) {
    let mut tabs = vec![
        EcsUiCameras::tab_from_world(world),
        EcsUiPlayerPlaceholder::tab_from_world(world),
        EcsUiSendChunkLoads::tab_from_world(world),
        EcsUiDebugSettings::tab_from_world(world),
    ];
    let total_tabs = tabs.len() as f32;

    let mut tree = egui_dock::Tree::new(vec![EcsUiTab::Viewport]);
    let [_viewport, mut side_bar] =
        tree.split_right(egui_dock::NodeIndex::root(), 0.7, vec![tabs.pop().unwrap()]);

    for (i, tab) in tabs.into_iter().enumerate() {
        side_bar = tree.split_below(side_bar, 1.0 / (total_tabs - i as f32), vec![tab])[1];
    }

    world.insert_resource(EcsUiState::new(tree));
}

#[derive(Resource, Default)]
struct GroundTexture {
    handle: Handle<Image>,
}

fn set_ground_sampler(
    mut ev_asset: EventReader<AssetEvent<Image>>,
    mut assets: ResMut<Assets<Image>>,
    map_img: Res<GroundTexture>,
    q_ground_plane: Query<&mut Handle<StandardMaterial>, With<GroundPlane>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
) {
    for ev in ev_asset.iter() {
        match ev {
            AssetEvent::Created { handle } => {
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

struct TransformLocal {
    model_transform: Transform,
}

impl Default for TransformLocal {
    fn default() -> Self {
        Self {
            model_transform: Transform::from_translation(Vec3::Y * 3.0),
        }
    }
}

fn orientation_gizmo_system(
    mut contexts: EguiContexts,
    q_camera: Query<(&Camera, &GlobalTransform)>,
    mut ev_align_view: EventWriter<AlignViewEvent>,
) {
    let (camera, camera_transform) = q_camera.get_single().unwrap();

    let viewport = if let Some(viewport) = &camera.viewport {
        Some(egui::Rect {
            min: egui::Pos2::from(viewport.physical_position.as_vec2().to_array()),
            max: egui::Pos2::from(
                (viewport.physical_position + viewport.physical_size)
                    .as_vec2()
                    .to_array(),
            ),
        })
    } else {
        None
    };
    let viewport = if let Some(viewport) = viewport {
        viewport
    } else {
        return;
    };

    egui::Area::new("Viewport")
        .fixed_pos((0.0, 0.0))
        .show(&contexts.ctx_mut(), |ui| {
            ui.with_layer_id(egui::LayerId::background(), |ui| {
                let painter = ui.painter();

                let padding =
                    egui::vec2(16.0, 16.0 + egui_dock::style::TabBarStyle::default().height);
                let radius = 24.0f32;
                // let center =
                //     (0.5 * viewport.min.to_vec2() + 0.5 * viewport.max.to_vec2()).to_pos2();

                let center = egui::pos2(
                    viewport.max.x - radius - padding.x,
                    viewport.min.y + radius + padding.y,
                );

                if let Some(pos) = ui.input(|input| input.pointer.hover_pos()) {
                    if (center - pos).length_sq() < (radius + 12.0) * (radius + 12.0) {
                        painter.circle_filled(
                            center,
                            radius + 12.0,
                            egui::Rgba::from_luminance_alpha(1.0, 0.2),
                        );
                    }
                };

                let inv_view_matrix = camera_transform.compute_matrix().inverse();

                let mut axis = [
                    (
                        "X",
                        Vec3::X,
                        EGUI_X_COLOR,
                        egui::Rgba::from_rgb(0.6, 0.3, 0.3),
                        true,
                    ),
                    (
                        "Y",
                        Vec3::Y,
                        EGUI_Y_COLOR,
                        egui::Rgba::from_rgb(0.3, 0.6, 0.3),
                        true,
                    ),
                    (
                        "Z",
                        Vec3::Z,
                        EGUI_Z_COLOR,
                        egui::Rgba::from_rgb(0.3, 0.3, 0.6),
                        true,
                    ),
                    (
                        "-X",
                        Vec3::NEG_X,
                        EGUI_X_COLOR,
                        egui::Rgba::from_rgb(0.6, 0.3, 0.3),
                        false,
                    ),
                    (
                        "-Y",
                        Vec3::NEG_Y,
                        EGUI_Y_COLOR,
                        egui::Rgba::from_rgb(0.3, 0.6, 0.3),
                        false,
                    ),
                    (
                        "-Z",
                        Vec3::NEG_Z,
                        EGUI_Z_COLOR,
                        egui::Rgba::from_rgb(0.3, 0.3, 0.6),
                        false,
                    ),
                ]
                .map(|data| {
                    (
                        data.0,
                        data.1,
                        inv_view_matrix * data.1.extend(0.0),
                        data.2,
                        data.3,
                        data.4,
                    )
                });
                axis.sort_by(|a, b| PartialOrd::partial_cmp(&a.2.z, &b.2.z).unwrap());

                for (letter, axis, screen_space_axis, color, secondary_color, primary) in axis {
                    let screen_space_axis =
                        egui::vec2(screen_space_axis.x, -screen_space_axis.y) * radius;
                    let screen_space_axis = center + screen_space_axis;

                    if primary {
                        painter.line_segment([center, screen_space_axis], (3.0, color));
                    }

                    let mut hovered = false;
                    if let (Some(pos), clicked) = ui
                        .input(|input| (input.pointer.hover_pos(), input.pointer.primary_clicked()))
                    {
                        if (screen_space_axis - pos).length_sq() < 6.0 * 6.0 {
                            if clicked {
                                ev_align_view.send(AlignViewEvent(-axis));
                            }
                            hovered = true;
                            painter.circle(screen_space_axis, 8.0, secondary_color, (2.0, color));
                            painter.text(
                                screen_space_axis + egui::vec2(1.0, 1.0),
                                egui::Align2::CENTER_CENTER,
                                letter,
                                egui::FontId {
                                    size: 10.,
                                    family: egui::FontFamily::Monospace,
                                },
                                egui::Color32::WHITE,
                            );
                        }
                    }

                    if !hovered {
                        if primary {
                            painter.circle_filled(screen_space_axis, 6.0, color);
                            painter.text(
                                screen_space_axis + egui::vec2(0.5, 0.5),
                                egui::Align2::CENTER_CENTER,
                                letter,
                                egui::FontId {
                                    size: 10.,
                                    family: egui::FontFamily::Monospace,
                                },
                                egui::Color32::BLACK,
                            );
                        } else {
                            painter.circle(screen_space_axis, 6.0, secondary_color, (2.0, color));
                        }
                    }
                }
            });
        });
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

mod constants;
