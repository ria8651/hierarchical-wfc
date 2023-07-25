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
use bevy_inspector_egui::bevy_inspector::{
    self, ui_for_entities_shared_components, ui_for_entity_with_children,
};
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
    // .add_plugins(render_pipeline::RenderPlugin)
    .add_systems(Update, (spritemap_fix, brush_system, camera_2d_system))
    .add_event::<BrushSelectEvent>()
    .add_plugins(SimpleTileMapPlugin)
    .add_plugins(Material2dPlugin::<BackgroundGridMaterial>::default())
    // .add_plugin(bevy_framepace::FramepacePlugin) // reduces input lag
    .add_plugins(DefaultInspectorConfigPlugin)
    .add_plugins(bevy_egui::EguiPlugin)
    // .add_plugins(bevy_mod_picking::plugins::DefaultPickingPlugins)
    .insert_resource(EditorState::new())
    .add_systems(Startup, setup)
    .add_systems(
        PostUpdate,
        show_ui_system
            .before(EguiSet::ProcessOutput)
            .before(bevy::transform::TransformSystem::TransformPropagate),
    )
    .add_systems(PostUpdate, set_camera_viewport.after(show_ui_system))
    .add_systems(Update, set_gizmo_mode)
    // .add_systems(Update, auto_add_raycast_target)
    // .add_systems(Update, handle_pick_events)
    .register_type::<Option<Handle<Image>>>()
    .register_type::<AlphaMode>();
    #[cfg(not(target_arch = "wasm32"))]
    {
        let settings = bevy_mod_debugdump::render_graph::Settings::default();
        let dot = bevy_mod_debugdump::render_graph_dot(&mut app, &settings);
        std::fs::write("render-graph.dot", dot).expect("Failed to write render-graph.dot");
    }
    app.run();
}

/*
fn auto_add_raycast_target(
    mut commands: Commands,
    query: Query<Entity, (Without<PickRaycastTarget>, With<Handle<Mesh>>)>,
) {
    for entity in &query {
        commands
            .entity(entity)
            .insert((PickRaycastTarget::default(), PickableBundle::default()));
    }
}

fn handle_pick_events(
    mut ui_state: ResMut<UiState>,
    mut click_events: EventReader<PointerClick>,
    mut egui: ResMut<EguiContext>,
    egui_entity: Query<&EguiPointer>,
) {
    let egui_context = egui.ctx_mut();

    for click in click_events.iter() {
        if egui_entity.get(click.target()).is_ok() {
            continue;
        };

        let modifiers = egui_context.input().modifiers;
        let add = modifiers.ctrl || modifiers.shift;

        ui_state
            .selected_entities
            .select_maybe_add(click.target(), add);
    }
}
*/

// make camera only render to view not obstructed by UI
fn set_camera_viewport(
    ui_state: Res<EditorState>,
    primary_window: Query<&mut Window, With<PrimaryWindow>>,
    egui_settings: Res<bevy_egui::EguiSettings>,
    mut cameras: Query<&mut Camera, With<MainCamera>>,
) {
    let mut cam = cameras.single_mut();

    let Ok(window) = primary_window.get_single() else {
        return;
    };

    let scale_factor = window.scale_factor() * egui_settings.scale_factor;

    let viewport_pos = ui_state.viewport_rect.left_top().to_vec2() * scale_factor as f32;
    let viewport_size = ui_state.viewport_rect.size() * scale_factor as f32;

    cam.viewport = Some(Viewport {
        physical_position: UVec2::new(viewport_pos.x as u32, viewport_pos.y as u32),
        physical_size: UVec2::new(viewport_size.x as u32, viewport_size.y as u32),
        depth: 0.0..1.0,
    });
}

#[derive(Default)]
struct ViewportAnchor {
    initial_world_translation: Vec3,
    initial_cursor_position: Vec2,
}

struct CameraSystemState {
    actual_zoom: f32,
    target_zoom: f32,
    anchor: Option<ViewportAnchor>,
}

impl Default for CameraSystemState {
    fn default() -> CameraSystemState {
        CameraSystemState {
            actual_zoom: 1.0,
            target_zoom: 1.0,
            anchor: None,
        }
    }
}

const ZOOM_SENSITIVITY: f32 = 0.25;

fn camera_2d_system(
    ui_state: Res<EditorState>,
    primary_window: Query<&mut Window, With<PrimaryWindow>>,
    mut state: Local<CameraSystemState>,
    mut camera_q: Query<
        (
            &Camera,
            &mut Transform,
            &GlobalTransform,
            &mut OrthographicProjection,
        ),
        With<MainCamera>,
    >,
    mut mouse_scroll_events: EventReader<MouseWheel>,
    buttons: Res<Input<MouseButton>>,
) {
    let (camera, mut camera_transform, global_camera_transform, mut projection) =
        camera_q.get_single_mut().unwrap();

    let cursor_position = match primary_window.get_single() {
        Ok(window) => window.cursor_position(),
        _ => None,
    };

    if let Some(pos) = cursor_position {
        if ui_state.viewport_rect.contains(egui::pos2(pos.x, pos.y)) {
            for event in mouse_scroll_events.iter() {
                let delta = match event.unit {
                    MouseScrollUnit::Line => event.y * 20.0,
                    MouseScrollUnit::Pixel => event.y,
                };
                state.target_zoom += ZOOM_SENSITIVITY * delta / 20.0
            }
            if buttons
                .get_pressed()
                .any(|button| button == &MouseButton::Middle)
            {
                let event_world_pos = camera
                    .viewport_to_world_2d(global_camera_transform, pos)
                    .unwrap();

                if state.anchor.is_none() {
                    state.anchor = Some(ViewportAnchor {
                        initial_world_translation: camera_transform.translation,
                        initial_cursor_position: pos,
                    });
                }
                let initial_event_world_pos = camera
                    .viewport_to_world_2d(
                        global_camera_transform,
                        state.anchor.as_ref().unwrap().initial_cursor_position,
                    )
                    .unwrap();

                camera_transform.translation =
                    state.anchor.as_ref().unwrap().initial_world_translation
                        - (event_world_pos - initial_event_world_pos).extend(0.0);
            }
        }
        if buttons.just_released(MouseButton::Middle) {
            state.anchor = None;
        }
        state.actual_zoom = state.actual_zoom * 0.98 + 0.02 * state.target_zoom;
        projection.scale = (state.actual_zoom).exp2();
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut custom_materials: ResMut<Assets<BackgroundGridMaterial>>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
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

    let texture_handle: Handle<Image> = asset_server.load("tilesets/images/cliffside.png");
    let texture_atlas =
        TextureAtlas::from_grid(texture_handle, Vec2::new(8.0, 8.0), 6, 15, None, None);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);

    // Set up tilemap
    let mut tilemap_bundle = TileMapBundle {
        texture_atlas: texture_atlas_handle.clone(),
        ..Default::default()
    };
    let mut tiles: Vec<(IVec3, Option<Tile>)> = Vec::new();
    tiles.push((
        IVec3::new(0, 0, 0),
        Some(Tile {
            sprite_index: 0,
            color: Color::WHITE,
            ..Default::default()
        }),
    ));
    tiles.push((
        IVec3::new(1, 0, 0),
        Some(Tile {
            sprite_index: 1,
            color: Color::WHITE,
            ..Default::default()
        }),
    ));

    // Perform tile update
    tilemap_bundle.tilemap.set_tiles(tiles);

    // Spawn tilemap
    commands.spawn(tilemap_bundle);

    commands.spawn((
        SpriteSheetBundle {
            sprite: TextureAtlasSprite {
                anchor: bevy::sprite::Anchor::Center,
                custom_size: Some(Vec2::new(8.0, 8.0)),
                color: Color::WHITE.with_a(0.5),
                index: 0,
                ..Default::default()
            },
            texture_atlas: texture_atlas_handle,
            ..Default::default()
        },
        Brush,
    ));
    let mut mesh = Mesh::new(bevy::render::render_resource::PrimitiveTopology::TriangleList);
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![[-1., -1., 0.], [3., -1., 0.], [-1., 3., 0.]],
    );

    commands.spawn((
        MaterialMesh2dBundle {
            mesh: meshes.add(mesh).into(),
            material: custom_materials.add(BackgroundGridMaterial { color: Color::RED }),
            transform: Transform::from_translation(Vec3::NEG_Z),
            ..Default::default()
        },
        NoFrustumCulling,
    ));
}

// https://stackoverflow.com/questions/76292957/what-is-the-correct-way-to-implement-nearestneighbor-for-textureatlas-sprites-in
fn spritemap_fix(mut ev_asset: EventReader<AssetEvent<Image>>, mut assets: ResMut<Assets<Image>>) {
    for ev in ev_asset.iter() {
        match ev {
            AssetEvent::Created { handle } => {
                if let Some(texture) = assets.get_mut(&handle) {
                    texture.sampler_descriptor = ImageSampler::nearest()
                }
            }
            _ => {}
        }
    }
}
