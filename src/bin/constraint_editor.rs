use std::any::TypeId;
use std::ops::{Add, Div, Mul};

use bevy::asset::{HandleId, ReflectAsset};
use bevy::core_pipeline::clear_color::ClearColorConfig;
use bevy::ecs::system::SystemState;

use bevy::prelude::{
    AssetEvent, Event, EventReader, EventWriter, GlobalTransform, IVec3, IntoSystemConfigs,
    MouseButton, Resource,
};
use bevy::render::texture::ImageSampler;
use bevy::render::Extract;
use bevy::sprite::{
    ColorMaterial, Material2dPlugin, Sprite, SpriteBundle, SpriteSheetBundle, TextureAtlas,
    TextureAtlasSprite,
};
use bevy::utils::{default, HashMap};
use bevy::winit::WinitWindows;
use bevy::{
    math::{UVec2, Vec2},
    prelude::{
        AlphaMode, App, AppTypeRegistry, AssetServer, Assets, Bundle, Camera, Camera2d,
        Camera2dBundle, Color, Commands, Component, DefaultPlugins, FromReflect, Handle, Image,
        Input, KeyCode, Mesh, OrthographicProjection, Plugin, PostUpdate, Query, Reflect,
        ReflectComponent, ReflectResource, Res, ResMut, Startup, Transform, Update, Vec3, With,
        World,
    },
};

use bevy_inspector_egui::bevy_egui::{self, egui, EguiContext, EguiUserTextures};
use bevy_inspector_egui::bevy_inspector::hierarchy::{hierarchy_ui, SelectedEntities};
use bevy_inspector_egui::bevy_inspector::{
    self, ui_for_entities_shared_components, ui_for_entity_with_children,
};
use bevy_inspector_egui::DefaultInspectorConfigPlugin;

use bevy::reflect::TypeRegistry;
use bevy::render::camera::{ScalingMode, Viewport};
use bevy::window::{CursorLeft, CursorMoved, PrimaryWindow, Window};
use bevy_inspector_egui::bevy_egui::EguiSet;
use bevy_inspector_egui::egui::TextureHandle;
use bevy_simple_tilemap::prelude::{SimpleTileMapPlugin, TileMapBundle};
use bevy_simple_tilemap::{Tile, TileMap};
use egui_dock::{DockArea, NodeIndex, Style, Tree};
use egui_gizmo::GizmoMode;
use wfc_lib::point_material::PointMaterial;

// use bevy_mod_picking::backends::egui::EguiPointer;
// use bevy_mod_picking::prelude::*;
fn main() {
    App::new()
        .add_plugins((DefaultPlugins, Material2dPlugin::<PointMaterial>::default()))
        .add_systems(Update, spritemap_fix)
        .add_systems(Update, brush_system)
        .add_event::<BrushSelectEvent>()
        .add_plugins(SimpleTileMapPlugin)
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
        .register_type::<AlphaMode>()
        .run();
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

#[derive(Default)]
struct EditorTilesets {
    unimported: Vec<UnimportedTilesetUiData>,
}

struct UnimportedTilesetUiData {
    texture_handle: egui::TextureHandle,
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
struct Brush {
    tile: BrushTile,
}

#[derive(Component)]
struct BrushTile;

#[derive(Event)]
struct BrushSelectEvent {
    pub tile: u32,
}

fn brush_system(
    ui_state: Res<UiState>,
    egui_settings: Res<bevy_egui::EguiSettings>,

    mut brush_select_event: EventReader<BrushSelectEvent>,
    mut brush_query: Query<(&mut Transform, &mut TextureAtlasSprite), With<Brush>>,
    mut cursor_move: EventReader<CursorMoved>,
    buttons: Res<Input<MouseButton>>,
    mut tile_map_q: Query<&mut TileMap>,

    primary_window: Query<&mut Window, With<PrimaryWindow>>,
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

    let buttons = buttons.get_just_released();
    for button in buttons {
        if button == &MouseButton::Left {
            dbg!(sprite.index);
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
                dbg!(active_tile.clone());
                dbg!(width);
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
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
    // let mut grid_wfc: GridWfc<BasicTileset> = GridWfc::new(UVec2::new(100, 100));
    // grid_wfc.collapse(1);

    // let tiles = match grid_wfc.validate() {
    //     Ok(tiles) => tiles,
    //     Err(e) => {
    //         error!("Error: {}", e);
    //         return;
    //     }
    // };

    // for y in (0..tiles[0].len()).rev() {
    //     for x in 0..tiles.len() {
    //         print!("{}", &tiles[x][y]);
    //     }
    //     println!();
    // }
    // let mut graph = PlanarGraph::new_voronoi(32, 32, 1.0);

    // graph.collapse(0);
    // graph.validate();

    // commands.spawn((
    //     MaterialMesh2dBundle {
    //         mesh: meshes.add(graph.mesh_edges()).into(),
    //         material: standard_materials.add(ColorMaterial {
    //             color: Color::hex("727272").unwrap(),
    //             ..Default::default()
    //         }),
    //         ..Default::default()
    //     },
    //     Wireframe,
    // ));

    // commands.spawn((
    //     MaterialMesh2dBundle {
    //         mesh: meshes.add(graph.mesh_nodes()).into(),
    //         material: custom_materials.add(PointMaterial {
    //             color: Color::WHITE,
    //         }),
    //         ..Default::default()
    //     },
    //     Wireframe,
    // ));

    commands.spawn((
        Camera2dBundle {
            projection: OrthographicProjection {
                scaling_mode: ScalingMode::AutoMin {
                    min_width: 256.0,
                    min_height: 256.0,
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
    ));

    // tileset
    // let mut tile_handles: Vec<Handle<Image>> = Vec::new();
    // for tile in 1..=16 {
    //     tile_handles.push(asset_server.load(format!("tileset/{}.png", tile).as_str()));
    // }

    // // result
    // for x in 0..tiles.len() {
    //     for y in 0..tiles[0].len() {
    //         let tile = tiles[x][y];
    //         if tile > 0 {
    //             let pos = Vec2::new(x as f32, y as f32);
    //             commands.spawn((
    //                 SpriteBundle {
    //                     texture: tile_handles[tile as usize - 1].clone(),
    //                     transform: Transform::from_translation(
    //                         ((pos + 0.5) / tiles.len() as f32 - 0.5).extend(0.0),
    //                     ),
    //                     sprite: Sprite {
    //                         custom_size: Some(Vec2::splat(1.0 / tiles.len() as f32)),
    //                         ..default()
    //                     },
    //                     ..default()
    //                 },
    //                 TileSprite,
    //             ));
    //         }
    //     }
    // }

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

    // let sprite = Sprite {
    //     color: Color::rgb(0.1, 0.1, 0.5),
    //     flip_x: false,
    //     flip_y: false,
    //     custom_size: Some(Vec2::new(100.0, 20.0)),
    //     anchor: Default::default(),
    //     ..Default::default()
    // };
    // let paddle = commands
    //     .spawn(SpriteBundle {
    //         sprite: sprite,
    //         transform: Transform::from_xyz(0.0, 0.0, 0.0),
    //         ..Default::default()
    //     })
    //     .id();

    // sprite: SpriteSheetBundle {
    //     sprite: TextureAtlasSprite {
    //         anchor: bevy::sprite::Anchor::Center,
    //         custom_size: Some(Vec2::new(32.0, 32.0)),
    //         color: Color::RED,
    //         index: 0,
    //         ..Default::default()
    //     },
    //     texture_atlas: texture_atlas_handle,
    //     ..Default::default()
    // },
    commands.spawn((
        Brush { tile: BrushTile },
        // SpriteBundle {
        //     sprite: Sprite {
        //         color: Color::rgb(0.5, 0.1, 0.1),
        //         flip_x: false,
        //         flip_y: false,
        //         custom_size: Some(Vec2::new(100.0, 20.0)),
        //         anchor: Default::default(),
        //         ..Default::default()
        //     },
        //     transform: Transform::from_xyz(0.0, 0.0, 0.0),
        //     ..Default::default()
        // },
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
