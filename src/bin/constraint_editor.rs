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
    .insert_resource(UiState::new())
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
#[derive(Component)]
struct MainCamera;

fn show_ui_system(world: &mut World) {
    let Ok(egui_context) = world
        .query_filtered::<&mut EguiContext, With<PrimaryWindow>>()
        .get_single(world)
    else {
        return;
    };
    let mut egui_context = egui_context.clone();

    world.resource_scope::<UiState, _>(|world, mut ui_state| {
        ui_state.ui(world, egui_context.get_mut())
    });
}

// make camera only render to view not obstructed by UI
fn set_camera_viewport(
    ui_state: Res<UiState>,
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

fn set_gizmo_mode(input: Res<Input<KeyCode>>, mut ui_state: ResMut<UiState>) {
    for (key, mode) in [
        (KeyCode::R, GizmoMode::Rotate),
        (KeyCode::T, GizmoMode::Translate),
        (KeyCode::S, GizmoMode::Scale),
    ] {
        if input.just_pressed(key) {
            ui_state.gizmo_mode = mode;
        }
    }
}

#[derive(Eq, PartialEq)]
enum InspectorSelection {
    Entities,
    Resource(TypeId, String),
    Asset(TypeId, String, HandleId),
}

#[derive(Resource)]
struct UiState {
    tree: Tree<EguiWindow>,
    viewport_rect: egui::Rect,
    selected_entities: SelectedEntities,
    selection: InspectorSelection,
    gizmo_mode: GizmoMode,
    tile_map_bundle: TileMapBundle,
    tile_size: u32,
    active_tile: Option<u32>,
}

impl UiState {
    pub fn new() -> Self {
        let mut tree = Tree::new(vec![EguiWindow::GameView]);
        let [game, _inspector] =
            tree.split_right(NodeIndex::root(), 0.75, vec![EguiWindow::Inspector]);
        let [game, _hierarchy] = tree.split_left(game, 0.2, vec![EguiWindow::Hierarchy]);
        let [_game, _bottom] = tree.split_below(
            game,
            0.8,
            vec![
                EguiWindow::Resources,
                EguiWindow::Assets,
                EguiWindow::TileMap,
            ],
        );

        Self {
            tree,
            selected_entities: SelectedEntities::default(),
            selection: InspectorSelection::Entities,
            viewport_rect: egui::Rect::NOTHING,
            gizmo_mode: GizmoMode::Translate,
            tile_size: 64,
            active_tile: None,
            tile_map_bundle: TileMapBundle::default(),
        }
    }

    fn ui(&mut self, world: &mut World, ctx: &mut egui::Context) {
        let mut tab_viewer = TabViewer {
            world,
            viewport_rect: &mut self.viewport_rect,
            selected_entities: &mut self.selected_entities,
            selection: &mut self.selection,
            gizmo_mode: self.gizmo_mode,
            tile_map_bundle: &self.tile_map_bundle,
            tile_size: &mut self.tile_size,
            active_tile: &mut self.active_tile,
        };
        DockArea::new(&mut self.tree)
            .style(Style::from_egui(ctx.style().as_ref()))
            .show(ctx, &mut tab_viewer);
    }
}

#[derive(Debug)]
enum EguiWindow {
    GameView,
    Hierarchy,
    Resources,
    Assets,
    Inspector,
    TileMap,
}

struct TabViewer<'a> {
    world: &'a mut World,
    selected_entities: &'a mut SelectedEntities,
    selection: &'a mut InspectorSelection,
    viewport_rect: &'a mut egui::Rect,
    gizmo_mode: GizmoMode,
    tile_map_bundle: &'a TileMapBundle,
    tile_size: &'a mut u32,
    active_tile: &'a mut Option<u32>,
}

impl egui_dock::TabViewer for TabViewer<'_> {
    type Tab = EguiWindow;

    fn ui(&mut self, ui: &mut egui_dock::egui::Ui, window: &mut Self::Tab) {
        let type_registry = self.world.resource::<AppTypeRegistry>().0.clone();

        match window {
            EguiWindow::GameView => {
                *self.viewport_rect = ui.clip_rect();

                draw_gizmo(ui, self.world, self.selected_entities, self.gizmo_mode);
            }
            EguiWindow::Hierarchy => {
                let selected = hierarchy_ui(self.world, ui, self.selected_entities);
                if selected {
                    *self.selection = InspectorSelection::Entities;
                }
            }
            EguiWindow::Resources => select_resource(ui, &type_registry, self.selection),
            EguiWindow::Assets => select_asset(ui, &type_registry, self.world, self.selection),
            EguiWindow::Inspector => match *self.selection {
                InspectorSelection::Entities => match self.selected_entities.as_slice() {
                    &[entity] => ui_for_entity_with_children(self.world, entity, ui),
                    entities => ui_for_entities_shared_components(self.world, entities, ui),
                },
                InspectorSelection::Resource(type_id, ref name) => {
                    ui.label(name);
                    bevy_inspector::by_type_id::ui_for_resource(
                        self.world,
                        type_id,
                        ui,
                        name,
                        &type_registry.read(),
                    )
                }
                InspectorSelection::Asset(type_id, ref name, handle) => {
                    ui.label(name);
                    bevy_inspector::by_type_id::ui_for_asset(
                        self.world,
                        type_id,
                        handle,
                        ui,
                        &type_registry.read(),
                    );
                }
            },
            EguiWindow::TileMap => tilemap_ui(self.world, ui, self.tile_size, self.active_tile),
        }
    }

    fn title(&mut self, window: &mut Self::Tab) -> egui_dock::egui::WidgetText {
        format!("{window:?}").into()
    }

    fn clear_background(&self, window: &Self::Tab) -> bool {
        !matches!(window, EguiWindow::GameView)
    }
}

#[derive(Component)]
struct Brush;

#[derive(Component)]
struct BrushTile;

#[derive(Event)]
struct BrushSelectEvent {
    pub tile: u32,
}

fn brush_system(
    ui_state: Res<UiState>,
    primary_window: Query<&mut Window, With<PrimaryWindow>>,
    egui_settings: Res<bevy_egui::EguiSettings>,

    mut brush_select_event: EventReader<BrushSelectEvent>,
    mut brush_query: Query<(&mut Transform, &mut TextureAtlasSprite), With<Brush>>,
    mut cursor_move: EventReader<CursorMoved>,
    buttons: Res<Input<MouseButton>>,
    mut tile_map_q: Query<&mut TileMap>,

    camera_q: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
) {
    let Ok(window) = primary_window.get_single() else {
        return;
    };

    let scale_factor = window.scale_factor() * egui_settings.scale_factor;

    let viewport_pos = ui_state.viewport_rect.left_top().to_vec2() * scale_factor as f32;
    let viewport_size = ui_state.viewport_rect.size() * scale_factor as f32;

    UVec2::new(viewport_pos.x as u32, viewport_pos.y as u32);
    UVec2::new(viewport_size.x as u32, viewport_size.y as u32);

    let (camera, camera_transform) = camera_q.single();

    let (mut transform, mut sprite) = brush_query.get_single_mut().unwrap();
    for event in brush_select_event.iter() {
        sprite.index = event.tile as usize;
    }
    for event in cursor_move.iter() {
        let cursor = event.position - Vec2::new(viewport_pos.x, viewport_pos.y);
        if let Some(world_space) = camera.viewport_to_world_2d(camera_transform, cursor) {
            transform.translation = world_space.extend(0.0).div(8.0).round().mul(8.0);
        }
    }
    if mouse_in_viewport(window, &ui_state) {
        for button in buttons.get_pressed() {
            if button == &MouseButton::Left {
                let mut tile_map: bevy::prelude::Mut<'_, TileMap> =
                    tile_map_q.get_single_mut().unwrap();
                tile_map.set_tile(
                    transform.translation.div(8.0).as_ivec3(),
                    Some(Tile {
                        sprite_index: sprite.index as u32,
                        ..Default::default()
                    }),
                )
            } else if button == &MouseButton::Right {
                let mut tile_map: bevy::prelude::Mut<'_, TileMap> =
                    tile_map_q.get_single_mut().unwrap();

                tile_map.set_tile(transform.translation.div(8.0).as_ivec3(), None)
            }
        }
    }
}

fn mouse_in_viewport(window: &Window, ui_state: &UiState) -> bool {
    if let Some(Vec2 { x: c_x, y: c_y }) = window.cursor_position() {
        if ui_state.viewport_rect.contains(egui::pos2(c_x, c_y)) {
            return true;
        }
    }
    return false;
}

fn tilemap_ui(
    world: &mut World,
    ui: &mut egui::Ui,
    tile_size: &mut u32,
    active_tile: &mut Option<u32>,
) {
    ui.label("tilemap");
    ui.add(egui::widgets::Slider::new(tile_size, 1..=128));
    let tile_size = tile_size.clone();

    // Definitely really bad
    let mut system_state: SystemState<(
        Res<Assets<TextureAtlas>>,
        ResMut<EguiUserTextures>,
        EventWriter<BrushSelectEvent>,
    )> = SystemState::new(world);

    let (textures, mut user_textures, mut brush_select_event) = system_state.get_mut(world);

    for atlas in textures.iter() {
        let texture_id = user_textures.add_image(atlas.1.texture.clone());
        let width = (ui.available_width() as u32).div_euclid(tile_size + 4);
        let (response, painter) = ui.allocate_painter(
            egui::Vec2::new(
                width as f32 * (tile_size as f32 + 4.0),
                (tile_size as f32 + 4.0)
                    * (atlas.1.textures.len() as f32 / width as f32).ceil() as f32,
            ),
            egui::Sense {
                click: true,
                drag: true,
                focusable: true,
            },
        );
        let size = atlas.1.size;
        for (i, texture_rect) in atlas.1.textures.iter().enumerate() {
            painter.image(
                texture_id,
                egui::Rect::from_min_max(
                    egui::Pos2::new(2.0, 2.0),
                    egui::Vec2::splat(tile_size as f32 + 2.0).to_pos2(),
                )
                .translate(
                    response.rect.min.to_vec2()
                        + egui::Vec2::new(
                            (tile_size as f32 + 4.0) * (i as u32 % width) as f32,
                            (tile_size as f32 + 4.0) * (i as u32).div_euclid(width) as f32,
                        ),
                ),
                egui::Rect::from_min_max(
                    egui::Pos2::new(texture_rect.min.x / size.x, texture_rect.min.y / size.y),
                    egui::Pos2::new(texture_rect.max.x / size.x, texture_rect.max.y / size.y),
                ),
                egui::Color32::WHITE,
            );
            // ui.add(egui::widgets::Image)
        }
        let tile_outer_size = tile_size as f32 + 4.0;
        if let Some(pos) = response.hover_pos() {
            let pos = (pos - response.rect.min).to_pos2();
            painter.rect_stroke(
                egui::Rect::from_min_max(
                    egui::pos2(
                        pos.x.div_euclid(tile_outer_size) * tile_outer_size + 1.0,
                        pos.y.div_euclid(tile_outer_size) * tile_outer_size + 1.0,
                    ),
                    egui::pos2(
                        pos.x.div_euclid(tile_outer_size).add(1.0) * tile_outer_size - 1.0,
                        pos.y.div_euclid(tile_outer_size).add(1.0) * tile_outer_size - 1.0,
                    ),
                )
                .translate(response.rect.min.to_vec2()),
                egui::Rounding::none(),
                egui::Stroke {
                    width: 2.0,
                    color: egui::Color32::from_rgb(0xad, 0xad, 0xad),
                },
            );

            if response.clicked() {
                let index = (pos.x / tile_outer_size).floor() as u32
                    + (pos.y / tile_outer_size).floor() as u32 * width;
                *active_tile = Some(index);
                brush_select_event.send(BrushSelectEvent { tile: index });
            }
        }
        if let Some(active_tile) = active_tile {
            painter.rect_stroke(
                egui::Rect::from_min_max(
                    egui::pos2(1.0, 1.0),
                    egui::pos2(tile_outer_size - 1.0, tile_outer_size - 1.0),
                )
                .translate(
                    egui::vec2(
                        (*active_tile % width) as f32,
                        active_tile.div_euclid(width) as f32,
                    ) * tile_outer_size
                        + response.rect.min.to_vec2(),
                ),
                egui::Rounding::none(),
                egui::Stroke {
                    width: 2.0,
                    color: egui::Color32::from_rgb(0xff, 0x61, 0x88),
                },
            );
        }
    }
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
    ui_state: Res<UiState>,
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

fn draw_gizmo(
    ui: &mut egui::Ui,
    world: &mut World,
    selected_entities: &SelectedEntities,
    gizmo_mode: GizmoMode,
) {
    // let (cam_transform, projection) = world
    //     .query_filtered::<(&GlobalTransform, &Projection), With<MainCamera>>()
    //     .single(world);
    // let view_matrix = Mat4::from(cam_transform.affine().inverse());
    // let projection_matrix = projection.get_projection_matrix();

    // if selected_entities.len() != 1 {
    //     return;
    // }

    // for selected in selected_entities.iter() {
    //     let Some(transform) = world.get::<Transform>(selected) else {
    //         continue;
    //     };
    //     let model_matrix = transform.compute_matrix();

    //     let Some(result) = Gizmo::new(selected)
    //         .model_matrix(model_matrix.to_cols_array_2d())
    //         .view_matrix(view_matrix.to_cols_array_2d())
    //         .projection_matrix(projection_matrix.to_cols_array_2d())
    //         .orientation(GizmoOrientation::Local)
    //         .mode(gizmo_mode)
    //         .interact(ui)
    //     else {
    //         continue;
    //     };

    //     let mut transform = world.get_mut::<Transform>(selected).unwrap();
    //     *transform = Transform {
    //         translation: Vec3::from(<[f32; 3]>::from(result.translation)),
    //         rotation: Quat::from_array(<[f32; 4]>::from(result.rotation)),
    //         scale: Vec3::from(<[f32; 3]>::from(result.scale)),
    //     };
    // }
}

fn select_resource(
    ui: &mut egui::Ui,
    type_registry: &TypeRegistry,
    selection: &mut InspectorSelection,
) {
    let mut resources: Vec<_> = type_registry
        .read()
        .iter()
        .filter(|registration| registration.data::<ReflectResource>().is_some())
        .map(|registration| (registration.short_name().to_owned(), registration.type_id()))
        .collect();
    resources.sort_by(|(name_a, _), (name_b, _)| name_a.cmp(name_b));

    for (resource_name, type_id) in resources {
        let selected = match *selection {
            InspectorSelection::Resource(selected, _) => selected == type_id,
            _ => false,
        };

        if ui.selectable_label(selected, &resource_name).clicked() {
            *selection = InspectorSelection::Resource(type_id, resource_name);
        }
    }
}

fn select_asset(
    ui: &mut egui::Ui,
    type_registry: &TypeRegistry,
    world: &World,
    selection: &mut InspectorSelection,
) {
    let type_registry = type_registry.read();
    let mut assets: Vec<_> = type_registry
        .iter()
        .filter_map(|registration| {
            let reflect_asset = registration.data::<ReflectAsset>()?;
            Some((
                registration.short_name().to_owned(),
                registration.type_id(),
                reflect_asset,
            ))
        })
        .collect();
    assets.sort_by(|(name_a, ..), (name_b, ..)| name_a.cmp(name_b));

    for (asset_name, asset_type_id, reflect_asset) in assets {
        let mut handles: Vec<_> = reflect_asset.ids(world).collect();
        handles.sort();

        ui.collapsing(format!("{asset_name} ({})", handles.len()), |ui| {
            for handle in handles {
                let selected = match *selection {
                    InspectorSelection::Asset(_, _, selected_id) => selected_id == handle,
                    _ => false,
                };

                if ui
                    .selectable_label(selected, format!("{:?}", handle))
                    .clicked()
                {
                    *selection =
                        InspectorSelection::Asset(asset_type_id, asset_name.clone(), handle);
                }
            }
        });
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
