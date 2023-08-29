use bevy::{asset::ChangeWatcher, window::PresentMode};
use bevy_render::texture::ImageSampler;
use passes::{
    facade_debug_system, facade_init_system, facade_mesh_system, layout_debug_system,
    layout_init_system,
};
use replay::replay_generation_system;
use std::time::Duration;
use tabs::*;

use bevy::{
    math::vec3,
    prelude::{AssetPlugin, PluginGroup, *},
    render::render_resource::{AddressMode, FilterMode, SamplerDescriptor},
};

use bevy::log::LogPlugin;
use bevy_inspector_egui::{bevy_egui, DefaultInspectorConfigPlugin};
use bevy_mod_billboard::prelude::*;
use bevy_mod_debugdump;
use bevy_rapier3d::prelude::{
    Collider, ComputedColliderShape, NoUserData, RapierPhysicsPlugin, RigidBody,
};
use hierarchical_wfc::{
    camera_plugin::cam_switcher::SwitchingCameraPlugin,
    materials::{debug_arc_material::DebugLineMaterial, tile_pbr_material::TilePbrMaterial},
    tools::MeshBuilder,
    ui_plugin::{EcsTab, EcsUiPlugin, EcsUiState, EcsUiTab},
    village::{
        facade_graph::{FacadePassData, FacadePassSettings, FacadeTileset},
        layout_graph::LayoutGraphSettings,
        layout_pass::LayoutTileset,
    },
    wfc::{
        bevy_passes::{
            wfc_collapse_system, wfc_ready_system, WfcEntityMarker, WfcFCollapsedData,
            WfcInitialData, WfcInvalidatedMarker, WfcParentPasses, WfcPassReadyMarker,
        },
        TileSet, WfcGraph,
    },
};
use rand::{rngs::StdRng, SeedableRng};

mod debug;
mod generation;
mod passes;
mod replay;
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
    let mut tabs = vec![
        EcsUiLayout::new(world),
        EcsUiCameras::new(world),
        EcsUiReplay::new(world),
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
