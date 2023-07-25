use std::any::TypeId;
use std::ops::{Add, Div, Mul};
use std::time::Duration;

use bevy::asset::{ChangeWatcher, HandleId, ReflectAsset};
use bevy::core_pipeline::clear_color::ClearColorConfig;
use bevy::ecs::system::SystemState;

use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::{
    AssetEvent, AssetPlugin, Event, EventReader, EventWriter, GlobalTransform, IVec3,
    IntoSystemConfigs, Local, MouseButton, PluginGroup, Resource,
};
use bevy::render::texture::ImageSampler;
use bevy::render::view::NoFrustumCulling;
use bevy::sprite::{
    Material2dPlugin, MaterialMesh2dBundle, SpriteSheetBundle, TextureAtlas, TextureAtlasSprite,
};
use bevy::{
    math::{UVec2, Vec2},
    prelude::*,
};

use bevy_inspector_egui::bevy_egui::{self, egui, EguiContext, EguiUserTextures};
use bevy_inspector_egui::bevy_inspector::hierarchy::{hierarchy_ui, SelectedEntities};

use bevy_inspector_egui::DefaultInspectorConfigPlugin;

use bevy::reflect::TypeRegistry;
use bevy::render::camera::{CameraRenderGraph, ScalingMode, Viewport};
use bevy::window::{CursorMoved, PresentMode, PrimaryWindow, Window, WindowPlugin};
use bevy_inspector_egui::bevy_egui::EguiSet;
use bevy_simple_tilemap::prelude::{SimpleTileMapPlugin, TileMapBundle};
use bevy_simple_tilemap::{Tile, TileMap};
use egui_dock::{DockArea, NodeIndex, Style, Tree};
use egui_gizmo::GizmoMode;
use wfc_lib::background_grid_material::BackgroundGridMaterial;
use wfc_lib::editor_ui::{
    brush_system, set_gizmo_mode, show_ui_system, Brush, BrushSelectEvent, EditorState, MainCamera,
};
use wfc_lib::point_material::PointMaterial;
use wfc_lib::render_pipeline::MainPassSettings;

// use bevy_mod_picking::backends::egui::EguiPointer;
// use bevy_mod_picking::prelude::*;
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
            .set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: PresentMode::Immediate,
                    ..Default::default()
                }),
                ..Default::default()
            }),
        Material2dPlugin::<PointMaterial>::default(),
    ))
    .add_plugins(bevy_egui::EguiPlugin)
    .add_systems(Startup, setup);
    app.run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera2dBundle {
            camera_render_graph: CameraRenderGraph::new("core_2d"),

            projection: OrthographicProjection {
                scaling_mode: ScalingMode::AutoMin {
                    min_width: 64.0,
                    min_height: 64.0,
                },
                ..Default::default()
            },
            tonemapping: bevy::core_pipeline::tonemapping::Tonemapping::None,
            camera_2d: Camera2d {
                clear_color: ClearColorConfig::Custom(Color::hex("2d2a2e").unwrap()),
                ..Default::default()
            },

            transform: Transform::from_translation(Vec3::new(0.5, 0.5, 2.0)),
            ..Default::default()
        },
        MainCamera,
        MainPassSettings {},
    ));
}
